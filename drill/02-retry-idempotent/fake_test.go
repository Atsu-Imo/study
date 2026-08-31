package payments

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"strings"
	"sync"
	"time"
)

// stubResponse は fakeTransport が1回の送信に対して返すもの。
// err が非 nil の場合は、送信そのものが失敗したことにする。
type stubResponse struct {
	status int
	body   string
	header http.Header
	err    error
}

func respOK(body string) stubResponse {
	return stubResponse{status: http.StatusOK, body: body}
}

func respStatus(code int, body string) stubResponse {
	return stubResponse{status: code, body: body}
}

// respRetryAfter は Retry-After 付きの 429 を返す。
func respRetryAfter(value string) stubResponse {
	return stubResponse{
		status: http.StatusTooManyRequests,
		body:   `{"code":"rate_limited","message":"リクエストが多すぎます"}`,
		header: http.Header{"Retry-After": []string{value}},
	}
}

// respErr は送信自体が失敗した状況（ネットワーク断など）を表す。
func respErr(err error) stubResponse {
	return stubResponse{err: err}
}

// recordedRequest は実際に送信されたリクエストの記録。
type recordedRequest struct {
	Method string
	URL    string
	Header http.Header
	Body   []byte
	Ctx    context.Context
}

// fakeTransport は http.RoundTripper のフェイク。
// responses を先頭から1つずつ返し、尽きたら最後のものを返し続ける。
type fakeTransport struct {
	responses []stubResponse

	mu       sync.Mutex
	requests []recordedRequest
	bodies   []*trackedBody
}

func (f *fakeTransport) RoundTrip(r *http.Request) (*http.Response, error) {
	var body []byte
	if r.Body != nil {
		b, err := io.ReadAll(r.Body)
		if err != nil {
			return nil, err
		}
		body = b
	}

	f.mu.Lock()
	n := len(f.requests)
	f.requests = append(f.requests, recordedRequest{
		Method: r.Method,
		URL:    r.URL.String(),
		Header: r.Header.Clone(),
		Body:   body,
		Ctx:    r.Context(),
	})
	f.mu.Unlock()

	stub := f.stubFor(n)
	if stub.err != nil {
		return nil, stub.err
	}

	tracked := &trackedBody{r: strings.NewReader(stub.body)}
	f.mu.Lock()
	f.bodies = append(f.bodies, tracked)
	f.mu.Unlock()

	header := stub.header.Clone()
	if header == nil {
		header = http.Header{}
	}
	if header.Get("Content-Type") == "" {
		header.Set("Content-Type", "application/json")
	}

	return &http.Response{
		StatusCode: stub.status,
		Status:     fmt.Sprintf("%d %s", stub.status, http.StatusText(stub.status)),
		Header:     header,
		Body:       tracked,
		Request:    r,
	}, nil
}

func (f *fakeTransport) stubFor(n int) stubResponse {
	if len(f.responses) == 0 {
		return respOK(`{"charge_id":"ch_default","status":"succeeded"}`)
	}
	if n >= len(f.responses) {
		return f.responses[len(f.responses)-1]
	}
	return f.responses[n]
}

// callCount は送信されたリクエストの本数。
func (f *fakeTransport) callCount() int {
	f.mu.Lock()
	defer f.mu.Unlock()
	return len(f.requests)
}

// recorded は送信されたリクエストの記録を送信順に返す。
func (f *fakeTransport) recorded() []recordedRequest {
	f.mu.Lock()
	defer f.mu.Unlock()
	return append([]recordedRequest(nil), f.requests...)
}

// idempotencyKeys は送信順に Idempotency-Key ヘッダの値を返す。
func (f *fakeTransport) idempotencyKeys() []string {
	keys := []string{}
	for _, r := range f.recorded() {
		keys = append(keys, r.Header.Get("Idempotency-Key"))
	}
	return keys
}

// unclosedBodies は返したのに Close されていないレスポンスボディの数。
func (f *fakeTransport) unclosedBodies() int {
	f.mu.Lock()
	defer f.mu.Unlock()

	n := 0
	for _, b := range f.bodies {
		if b.closeCount() == 0 {
			n++
		}
	}
	return n
}

// handedOutBodies は返したレスポンスボディの数。
func (f *fakeTransport) handedOutBodies() int {
	f.mu.Lock()
	defer f.mu.Unlock()
	return len(f.bodies)
}

// trackedBody は Close されたかどうかを記録するレスポンスボディ。
type trackedBody struct {
	r io.Reader

	mu     sync.Mutex
	closed int
}

func (b *trackedBody) Read(p []byte) (int, error) {
	return b.r.Read(p)
}

func (b *trackedBody) Close() error {
	b.mu.Lock()
	defer b.mu.Unlock()
	b.closed++
	return nil
}

func (b *trackedBody) closeCount() int {
	b.mu.Lock()
	defer b.mu.Unlock()
	return b.closed
}

// fakeSleeper は実時間を消費せず、待機の長さだけを記録する Sleeper。
type fakeSleeper struct {
	mu    sync.Mutex
	slept []time.Duration

	// onSleep が非 nil なら、記録後に呼ばれてその戻り値が Sleep の戻り値になる。
	onSleep func(ctx context.Context, d time.Duration) error
}

func (s *fakeSleeper) Sleep(ctx context.Context, d time.Duration) error {
	s.mu.Lock()
	s.slept = append(s.slept, d)
	hook := s.onSleep
	s.mu.Unlock()

	if hook != nil {
		return hook(ctx, d)
	}

	// 実時間は進めないが、すでに終了している ctx は尊重する。
	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
		return nil
	}
}

// durations は待機した長さを待機順に返す。
func (s *fakeSleeper) durations() []time.Duration {
	s.mu.Lock()
	defer s.mu.Unlock()
	return append([]time.Duration(nil), s.slept...)
}

// keySeq は連番の冪等キーを発行する。どの試行がどのキーを使ったか追える。
func keySeq() func() string {
	var mu sync.Mutex
	n := 0

	return func() string {
		mu.Lock()
		defer mu.Unlock()
		n++
		return fmt.Sprintf("key-%d", n)
	}
}
