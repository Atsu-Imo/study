package payments

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"time"
)

// chargesPath は課金エンドポイントのパス。
const chargesPath = "/v1/charges"

// Charge は課金を1件依頼する。
//
// 一時的な失敗は再試行し、その際に二重課金が起きないようにする。
// 満たすべき要件は README.md を参照。
func (c *Client) Charge(ctx context.Context, req ChargeRequest) (*ChargeResult, error) {
	// もう誰も結果を必要としていないなら、金の動く操作は投げない。
	if err := ctx.Err(); err != nil {
		return nil, err
	}

	payload, err := json.Marshal(chargeBody{
		OrderID:   req.OrderID,
		AmountJPY: req.AmountJPY,
	})
	if err != nil {
		return nil, fmt.Errorf("課金リクエストを組み立てられない: %w", err)
	}

	// 冪等キーは1回の Charge につき1つ。再試行でも同じものを送る。
	return c.sendWithRetry(ctx, payload, c.NewIdempotencyKey())
}

// sendWithRetry は再試行の方針だけを持つ。HTTP の詳細は attempt が知っている。
func (c *Client) sendWithRetry(ctx context.Context, payload []byte, key string) (*ChargeResult, error) {
	var lastErr error

	for i := 0; i < c.MaxAttempts; i++ {
		result, advice, err := c.attempt(ctx, payload, key)
		if err == nil {
			return result, nil
		}
		if !advice.retry {
			return nil, err
		}
		lastErr = err

		// 最後の試行の後は待たない。
		if i == c.MaxAttempts-1 {
			break
		}

		wait := advice.after
		if wait == 0 {
			wait = c.backoff(i)
		}
		if err := c.Sleeper.Sleep(ctx, wait); err != nil {
			return nil, err
		}
	}

	return nil, fmt.Errorf("%w: %w", ErrRetryExhausted, lastErr)
}

// backoff は i 回目 (0 始まり) の失敗の後に待つ時間を返す。
func (c *Client) backoff(i int) time.Duration {
	d := c.BaseDelay << i
	// 桁あふれで負や 0 になった場合も上限に丸める。
	if d > c.MaxDelay || d <= 0 {
		return c.MaxDelay
	}
	return d
}

// retryAdvice は失敗した試行を再試行してよいか、してよいならどれだけ待つかを表す。
// after が 0 ならバックオフに従う。
type retryAdvice struct {
	retry bool
	after time.Duration
}

// attempt は1回だけ送り、結果を「成功 / 再試行してよい失敗 / 打ち切る失敗」に分類する。
func (c *Client) attempt(ctx context.Context, payload []byte, key string) (*ChargeResult, retryAdvice, error) {
	// 再試行のたびにボディを最初から読み直せるよう、Reader は毎回作る。
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.BaseURL+chargesPath, bytes.NewReader(payload))
	if err != nil {
		return nil, retryAdvice{}, err
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Idempotency-Key", key)

	res, err := c.HTTPClient.Do(req)
	if err != nil {
		// 送信できていないので、同じ冪等キーで送り直してよい。
		return nil, retryAdvice{retry: true}, err
	}

	body, err := io.ReadAll(res.Body)
	res.Body.Close()
	if err != nil {
		return nil, retryAdvice{retry: true}, err
	}

	switch {
	case res.StatusCode == http.StatusOK:
		var result ChargeResult
		if err := json.Unmarshal(body, &result); err != nil {
			return nil, retryAdvice{}, fmt.Errorf("成功レスポンスを読めない: %w", err)
		}
		return &result, retryAdvice{}, nil

	case res.StatusCode == http.StatusTooManyRequests:
		return nil, retryAdvice{retry: true, after: retryAfter(res.Header)}, apiError(res.StatusCode, body)

	case res.StatusCode >= 500:
		return nil, retryAdvice{retry: true}, apiError(res.StatusCode, body)

	default:
		// 4xx は何度送っても結果が変わらない。想定外のステータスもここで打ち切る。
		return nil, retryAdvice{}, apiError(res.StatusCode, body)
	}
}

// apiError はエラーレスポンスを *APIError にする。
// ボディが JSON でなくても、ステータスコードだけは呼び出し元に伝える。
func apiError(status int, body []byte) *APIError {
	apiErr := &APIError{StatusCode: status}
	_ = json.Unmarshal(body, apiErr)
	return apiErr
}

// retryAfter は Retry-After ヘッダの秒数を返す。読めなければ 0。
func retryAfter(h http.Header) time.Duration {
	v := h.Get("Retry-After")
	if v == "" {
		return 0
	}
	// HTTP-date 形式には対応しない。読めなければバックオフに従う。
	secs, err := strconv.Atoi(v)
	if err != nil || secs <= 0 {
		return 0
	}
	return time.Duration(secs) * time.Second
}
