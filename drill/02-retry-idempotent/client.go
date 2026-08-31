// Package payments は決済プロバイダ Stellapay への課金リクエストを送る。
//
// Stellapay は混雑時に 429 を返し、まれに 5xx を返す。ネットワーク断も起きる。
// 単純に再送すると二重課金になるため、冪等キーと再試行をセットで扱うのが
// このパッケージの責務。社内の注文サービスから呼ばれている。
package payments

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"fmt"
	"net/http"
	"time"
)

// NewClient で埋める既定値。
const (
	DefaultMaxAttempts = 4
	DefaultBaseDelay   = 200 * time.Millisecond
	DefaultMaxDelay    = 5 * time.Second
)

// ChargeRequest は課金1件の依頼。
type ChargeRequest struct {
	OrderID   string
	AmountJPY int64
}

// chargeBody は POST /v1/charges に送る JSON ボディ。
type chargeBody struct {
	OrderID   string `json:"order_id"`
	AmountJPY int64  `json:"amount_jpy"`
}

// ChargeResult は課金が確定したときにプロバイダが返す内容。
type ChargeResult struct {
	ChargeID string `json:"charge_id"`
	Status   string `json:"status"`
}

// APIError はプロバイダがエラーレスポンスとして返した内容。
//
// 呼び出し元は Code を見て「カード拒否」「残高不足」などの
// ユーザー向けメッセージを出し分ける。
type APIError struct {
	StatusCode int    `json:"-"`
	Code       string `json:"code"`
	Message    string `json:"message"`
}

func (e *APIError) Error() string {
	return fmt.Sprintf("stellapay: status %d: %s: %s", e.StatusCode, e.Code, e.Message)
}

// ErrRetryExhausted は再試行の上限まで試しても成功しなかったことを表す。
//
// 上位はこれを見て決済状態を「不明」として保留にし、後続の照合バッチに回す。
var ErrRetryExhausted = errors.New("retry exhausted")

// Sleeper は再試行前の待機を抽象化する。テストが実時間を消費しないための境界。
type Sleeper interface {
	// Sleep は d だけ待つ。待っている間に ctx が終了したら ctx.Err() を返す。
	Sleep(ctx context.Context, d time.Duration) error
}

// RealSleeper は time を使う本番用の Sleeper。
type RealSleeper struct{}

func (RealSleeper) Sleep(ctx context.Context, d time.Duration) error {
	timer := time.NewTimer(d)
	defer timer.Stop()

	select {
	case <-timer.C:
		return nil
	case <-ctx.Done():
		return ctx.Err()
	}
}

// Client は Stellapay への課金クライアント。複数の goroutine から同時に使える。
//
// 必ず NewClient で組み立てること。設定の解決はそこで一度だけ行うので、
// リクエストのたびに未設定を気にする必要はない。
type Client struct {
	// BaseURL はプロバイダのエンドポイント。末尾にスラッシュは付けない。
	BaseURL string

	// HTTPClient は送信に使う。
	HTTPClient *http.Client

	// Sleeper は再試行前の待機に使う。
	Sleeper Sleeper

	// NewIdempotencyKey は冪等キーを1つ発行する。
	NewIdempotencyKey func() string

	// MaxAttempts は初回を含む最大試行回数。
	MaxAttempts int

	// BaseDelay はバックオフの基準時間。
	BaseDelay time.Duration

	// MaxDelay は1回あたりの待機時間の上限。
	MaxDelay time.Duration
}

// Option は NewClient に渡す設定。
type Option func(*Client)

// NewClient は課金クライアントを組み立てる。
//
// 既定値の解決はここで一度だけ行う。アプリケーションの初期化時に1つ作って
// 使い回すことを想定している。
func NewClient(baseURL string, opts ...Option) *Client {
	c := &Client{
		BaseURL:           baseURL,
		HTTPClient:        http.DefaultClient,
		Sleeper:           RealSleeper{},
		NewIdempotencyKey: NewRandomKey,
		MaxAttempts:       DefaultMaxAttempts,
		BaseDelay:         DefaultBaseDelay,
		MaxDelay:          DefaultMaxDelay,
	}
	for _, opt := range opts {
		opt(c)
	}
	return c
}

// WithHTTPClient は送信に使う *http.Client を差し替える。
func WithHTTPClient(hc *http.Client) Option {
	return func(c *Client) { c.HTTPClient = hc }
}

// WithSleeper は待機の実装を差し替える。テストで実時間を消費しないために使う。
func WithSleeper(s Sleeper) Option {
	return func(c *Client) { c.Sleeper = s }
}

// WithIdempotencyKeyFunc は冪等キーの発行方法を差し替える。
func WithIdempotencyKeyFunc(fn func() string) Option {
	return func(c *Client) { c.NewIdempotencyKey = fn }
}

// WithMaxAttempts は初回を含む最大試行回数を設定する。
func WithMaxAttempts(n int) Option {
	return func(c *Client) { c.MaxAttempts = n }
}

// WithBackoff はバックオフの基準時間と1回あたりの上限を設定する。
func WithBackoff(base, max time.Duration) Option {
	return func(c *Client) {
		c.BaseDelay = base
		c.MaxDelay = max
	}
}

// NewRandomKey はランダムな冪等キーを発行する。
func NewRandomKey() string {
	var b [16]byte
	if _, err := rand.Read(b[:]); err != nil {
		// crypto/rand は実質失敗しない。失敗する状況ではプロセスを続ける意味がない。
		panic(err)
	}
	return hex.EncodeToString(b[:])
}
