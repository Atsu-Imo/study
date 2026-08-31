package payments

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"slices"
	"testing"
	"time"
)

const (
	testBaseURL = "https://stellapay.test"
	successBody = `{"charge_id":"ch_123","status":"succeeded"}`
	serverBody  = `{"code":"internal_error","message":"一時的な障害です"}`
)

// errNetworkDown は送信自体が失敗する状況を表すテスト用のエラー。
var errNetworkDown = errors.New("network down")

// testCtxKey は ctx がリクエストまで届いているかを確認するためのキー。
type testCtxKey struct{}

func newTestClient(tr *fakeTransport, sl *fakeSleeper, opts ...Option) *Client {
	base := []Option{
		WithHTTPClient(&http.Client{Transport: tr}),
		WithSleeper(sl),
		WithIdempotencyKeyFunc(keySeq()),
		WithMaxAttempts(3),
		WithBackoff(100*time.Millisecond, 5*time.Second),
	}
	return NewClient(testBaseURL, append(base, opts...)...)
}

func decodeBody(t *testing.T, raw []byte) chargeBody {
	t.Helper()

	var body chargeBody
	if err := json.Unmarshal(raw, &body); err != nil {
		t.Fatalf("送信ボディが JSON として読めない (%q): %v", string(raw), err)
	}
	return body
}

// 1. 1回目で成功したら、結果を返して再試行も待機もしない。
func TestCharge_Success(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{respOK(successBody)}}
	sl := &fakeSleeper{}
	c := newTestClient(tr, sl)

	res, err := c.Charge(context.Background(), ChargeRequest{OrderID: "order-1", AmountJPY: 4980})
	if err != nil {
		t.Fatalf("エラーは返らないはず: %v", err)
	}
	if res == nil {
		t.Fatal("結果が nil")
	}
	if res.ChargeID != "ch_123" {
		t.Errorf("ChargeID = %q, want %q", res.ChargeID, "ch_123")
	}
	if res.Status != "succeeded" {
		t.Errorf("Status = %q, want %q", res.Status, "succeeded")
	}
	if tr.callCount() != 1 {
		t.Errorf("送信回数 = %d, want 1 (成功したらそれ以上送らない)", tr.callCount())
	}
	if got := sl.durations(); len(got) != 0 {
		t.Errorf("成功時に待機してはいけない: %v", got)
	}
}

// 2. 送信されるリクエストの形。
func TestCharge_RequestShape(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{respOK(successBody)}}
	c := newTestClient(tr, &fakeSleeper{})

	ctx := context.WithValue(context.Background(), testCtxKey{}, "traced")
	if _, err := c.Charge(ctx, ChargeRequest{OrderID: "order-1", AmountJPY: 4980}); err != nil {
		t.Fatalf("エラーは返らないはず: %v", err)
	}

	got := tr.recorded()
	if len(got) != 1 {
		t.Fatalf("送信回数 = %d, want 1", len(got))
	}
	req := got[0]

	if req.Method != http.MethodPost {
		t.Errorf("メソッド = %q, want %q", req.Method, http.MethodPost)
	}
	if want := testBaseURL + "/v1/charges"; req.URL != want {
		t.Errorf("URL = %q, want %q", req.URL, want)
	}
	if ct := req.Header.Get("Content-Type"); ct != "application/json" {
		t.Errorf("Content-Type = %q, want %q", ct, "application/json")
	}
	if key := req.Header.Get("Idempotency-Key"); key != "key-1" {
		t.Errorf("Idempotency-Key = %q, want %q (NewIdempotencyKey で発行した値を送ること)", key, "key-1")
	}

	body := decodeBody(t, req.Body)
	if body.OrderID != "order-1" {
		t.Errorf("ボディの order_id = %q, want %q", body.OrderID, "order-1")
	}
	if body.AmountJPY != 4980 {
		t.Errorf("ボディの amount_jpy = %d, want 4980", body.AmountJPY)
	}

	if req.Ctx.Value(testCtxKey{}) != "traced" {
		t.Error("呼び出し元の ctx がリクエストに紐づいていない")
	}
}

// 3. 5xx は再試行する。指数バックオフで待つ。
func TestCharge_RetriesOnServerError(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{
		respStatus(http.StatusServiceUnavailable, serverBody),
		respStatus(http.StatusInternalServerError, serverBody),
		respOK(successBody),
	}}
	sl := &fakeSleeper{}
	c := newTestClient(tr, sl)

	res, err := c.Charge(context.Background(), ChargeRequest{OrderID: "order-1", AmountJPY: 4980})
	if err != nil {
		t.Fatalf("3回目で成功するので、エラーは返らないはず: %v", err)
	}
	if res == nil || res.ChargeID != "ch_123" {
		t.Fatalf("成功したレスポンスを返すこと: %+v", res)
	}
	if tr.callCount() != 3 {
		t.Errorf("送信回数 = %d, want 3", tr.callCount())
	}

	want := []time.Duration{100 * time.Millisecond, 200 * time.Millisecond}
	if got := sl.durations(); !slices.Equal(got, want) {
		t.Errorf("待機時間 = %v, want %v (BaseDelay から倍々)", got, want)
	}
}

// 4. 4xx は再試行しない。APIError として返す。
func TestCharge_NoRetryOnClientError(t *testing.T) {
	body := `{"code":"card_declined","message":"カードが拒否されました"}`
	tr := &fakeTransport{responses: []stubResponse{respStatus(http.StatusPaymentRequired, body)}}
	sl := &fakeSleeper{}
	c := newTestClient(tr, sl)

	res, err := c.Charge(context.Background(), ChargeRequest{OrderID: "order-1", AmountJPY: 4980})
	if err == nil {
		t.Fatal("402 はエラーとして返すこと")
	}
	if res != nil {
		t.Errorf("エラー時は結果を返さないこと: %+v", res)
	}
	if tr.callCount() != 1 {
		t.Errorf("送信回数 = %d, want 1 (カード拒否は何度送っても結果が変わらない)", tr.callCount())
	}
	if got := sl.durations(); len(got) != 0 {
		t.Errorf("再試行しないので待機もしないはず: %v", got)
	}
	if errors.Is(err, ErrRetryExhausted) {
		t.Errorf("再試行していないので ErrRetryExhausted にはならない: %v", err)
	}

	var apiErr *APIError
	if !errors.As(err, &apiErr) {
		t.Fatalf("errors.As(err, **APIError) が false: %v", err)
	}
	if apiErr.StatusCode != http.StatusPaymentRequired {
		t.Errorf("APIError.StatusCode = %d, want 402", apiErr.StatusCode)
	}
	if apiErr.Code != "card_declined" {
		t.Errorf("APIError.Code = %q, want %q (ボディの内容を読むこと)", apiErr.Code, "card_declined")
	}
	if apiErr.Message != "カードが拒否されました" {
		t.Errorf("APIError.Message = %q が入っていない", apiErr.Message)
	}
}

// 5. 送信自体が失敗した場合も再試行する。
func TestCharge_RetriesOnTransportError(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{
		respErr(errNetworkDown),
		respOK(successBody),
	}}
	sl := &fakeSleeper{}
	c := newTestClient(tr, sl)

	if _, err := c.Charge(context.Background(), ChargeRequest{OrderID: "order-1", AmountJPY: 4980}); err != nil {
		t.Fatalf("2回目で成功するので、エラーは返らないはず: %v", err)
	}
	if tr.callCount() != 2 {
		t.Errorf("送信回数 = %d, want 2", tr.callCount())
	}
	want := []time.Duration{100 * time.Millisecond}
	if got := sl.durations(); !slices.Equal(got, want) {
		t.Errorf("待機時間 = %v, want %v", got, want)
	}
}

// 6. 429 は再試行する。Retry-After が読めればバックオフより優先する。
func TestCharge_RespectsRetryAfter(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{
		respRetryAfter("2"),
		respRetryAfter("banana"), // 読めない値
		respOK(successBody),
	}}
	sl := &fakeSleeper{}
	c := newTestClient(tr, sl, WithMaxAttempts(4))

	if _, err := c.Charge(context.Background(), ChargeRequest{OrderID: "order-1", AmountJPY: 4980}); err != nil {
		t.Fatalf("3回目で成功するので、エラーは返らないはず: %v", err)
	}
	if tr.callCount() != 3 {
		t.Errorf("送信回数 = %d, want 3 (429 は再試行する)", tr.callCount())
	}

	// 1回目は Retry-After の 2 秒、2回目は値が読めないので通常のバックオフ。
	want := []time.Duration{2 * time.Second, 200 * time.Millisecond}
	if got := sl.durations(); !slices.Equal(got, want) {
		t.Errorf("待機時間 = %v, want %v", got, want)
	}
}

// 7. 同じ呼び出しの再試行では同じ冪等キーを送り、別の呼び出しでは別のキーを使う。
func TestCharge_IdempotencyKeyPerCall(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{
		respStatus(http.StatusServiceUnavailable, serverBody),
		respStatus(http.StatusServiceUnavailable, serverBody),
		respOK(successBody),
	}}
	c := newTestClient(tr, &fakeSleeper{})

	if _, err := c.Charge(context.Background(), ChargeRequest{OrderID: "order-1", AmountJPY: 4980}); err != nil {
		t.Fatalf("3回目で成功するので、エラーは返らないはず: %v", err)
	}

	keys := tr.idempotencyKeys()
	if len(keys) != 3 {
		t.Fatalf("送信回数 = %d, want 3", len(keys))
	}
	for i, k := range keys {
		if k != keys[0] {
			t.Fatalf("再試行で冪等キーが変わっている (%d本目 = %q, 1本目 = %q)。二重課金になる", i+1, k, keys[0])
		}
	}

	// 2回目の呼び出しは別の課金なので、別のキーでなければならない。
	if _, err := c.Charge(context.Background(), ChargeRequest{OrderID: "order-2", AmountJPY: 1200}); err != nil {
		t.Fatalf("エラーは返らないはず: %v", err)
	}
	after := tr.idempotencyKeys()
	if len(after) != 4 {
		t.Fatalf("送信回数 = %d, want 4", len(after))
	}
	if after[3] == keys[0] {
		t.Errorf("別の呼び出しなのに冪等キーが同じ (%q)。2件目の課金が握り潰される", after[3])
	}
}

// 8. 再試行のたびにボディを送り直す。
func TestCharge_ResendsBodyOnRetry(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{
		respStatus(http.StatusServiceUnavailable, serverBody),
		respOK(successBody),
	}}
	c := newTestClient(tr, &fakeSleeper{})

	if _, err := c.Charge(context.Background(), ChargeRequest{OrderID: "order-1", AmountJPY: 4980}); err != nil {
		t.Fatalf("2回目で成功するので、エラーは返らないはず: %v", err)
	}

	got := tr.recorded()
	if len(got) != 2 {
		t.Fatalf("送信回数 = %d, want 2", len(got))
	}
	for i, req := range got {
		if len(req.Body) == 0 {
			t.Fatalf("%d本目のボディが空。1本目で読み切った Reader をそのまま使い回していないか", i+1)
		}
		body := decodeBody(t, req.Body)
		if body.OrderID != "order-1" || body.AmountJPY != 4980 {
			t.Errorf("%d本目のボディ = %+v, want {order-1 4980}", i+1, body)
		}
	}
}

// 9. バックオフは指数的に増え、MaxDelay で頭打ちになる。
func TestCharge_BackoffIsCapped(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{respStatus(http.StatusInternalServerError, serverBody)}}
	sl := &fakeSleeper{}
	c := newTestClient(tr, sl,
		WithMaxAttempts(5),
		WithBackoff(100*time.Millisecond, 250*time.Millisecond),
	)

	if _, err := c.Charge(context.Background(), ChargeRequest{OrderID: "order-1", AmountJPY: 4980}); err == nil {
		t.Fatal("ずっと 500 なのでエラーを返すこと")
	}

	want := []time.Duration{
		100 * time.Millisecond,
		200 * time.Millisecond,
		250 * time.Millisecond,
		250 * time.Millisecond,
	}
	if got := sl.durations(); !slices.Equal(got, want) {
		t.Errorf("待機時間 = %v, want %v (5回試行なら待機は4回。最後の失敗の後は待たない)", got, want)
	}
}

// 10. 上限まで試して失敗したら、ErrRetryExhausted と最後の原因の両方を辿れる。
func TestCharge_Exhausted(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{respStatus(http.StatusInternalServerError, serverBody)}}
	sl := &fakeSleeper{}
	c := newTestClient(tr, sl)

	res, err := c.Charge(context.Background(), ChargeRequest{OrderID: "order-1", AmountJPY: 4980})
	if err == nil {
		t.Fatal("ずっと 500 なのでエラーを返すこと")
	}
	if res != nil {
		t.Errorf("エラー時は結果を返さないこと: %+v", res)
	}
	if tr.callCount() != 3 {
		t.Errorf("送信回数 = %d, want 3 (MaxAttempts は初回を含む)", tr.callCount())
	}
	if got := sl.durations(); len(got) != 2 {
		t.Errorf("待機回数 = %d, want 2 (最後の失敗の後は待たない): %v", len(got), got)
	}
	if !errors.Is(err, ErrRetryExhausted) {
		t.Errorf("errors.Is(err, ErrRetryExhausted) が false: %v", err)
	}

	var apiErr *APIError
	if !errors.As(err, &apiErr) {
		t.Fatalf("最後の失敗の原因が辿れない。wrap すること: %v", err)
	}
	if apiErr.StatusCode != http.StatusInternalServerError {
		t.Errorf("APIError.StatusCode = %d, want 500", apiErr.StatusCode)
	}
}

// 11. 送信自体が失敗し続けた場合も、原因のエラーを辿れる。
func TestCharge_ExhaustedKeepsTransportError(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{respErr(errNetworkDown)}}
	c := newTestClient(tr, &fakeSleeper{})

	_, err := c.Charge(context.Background(), ChargeRequest{OrderID: "order-1", AmountJPY: 4980})
	if err == nil {
		t.Fatal("ずっと送信に失敗するのでエラーを返すこと")
	}
	if !errors.Is(err, ErrRetryExhausted) {
		t.Errorf("errors.Is(err, ErrRetryExhausted) が false: %v", err)
	}
	if !errors.Is(err, errNetworkDown) {
		t.Errorf("原因のエラーが辿れない。wrap すること: %v", err)
	}
}

// 12. 待機中に ctx がキャンセルされたら、次の試行に進まず速やかに返る。
func TestCharge_CancelDuringBackoff(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{
		respStatus(http.StatusServiceUnavailable, serverBody),
		respOK(successBody),
	}}

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	sl := &fakeSleeper{}
	sl.onSleep = func(ctx context.Context, _ time.Duration) error {
		// 待機に入った瞬間に呼び出し元がキャンセルした状況。
		cancel()
		return ctx.Err()
	}
	c := newTestClient(tr, sl)

	_, err := c.Charge(ctx, ChargeRequest{OrderID: "order-1", AmountJPY: 4980})
	if err == nil {
		t.Fatal("キャンセルされたらエラーを返すこと")
	}
	if !errors.Is(err, context.Canceled) {
		t.Errorf("errors.Is(err, context.Canceled) が false: %v", err)
	}
	if tr.callCount() != 1 {
		t.Errorf("送信回数 = %d, want 1 (待機が中断されたら次の試行に進まない)", tr.callCount())
	}
}

// 13. 呼び出し時点で ctx が終了していたら、1本も送らない。
func TestCharge_AlreadyCanceledContext(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{respOK(successBody)}}
	c := newTestClient(tr, &fakeSleeper{})

	ctx, cancel := context.WithCancel(context.Background())
	cancel()

	_, err := c.Charge(ctx, ChargeRequest{OrderID: "order-1", AmountJPY: 4980})
	if err == nil {
		t.Fatal("終了済みの ctx ならエラーを返すこと")
	}
	if !errors.Is(err, context.Canceled) {
		t.Errorf("errors.Is(err, context.Canceled) が false: %v", err)
	}
	if tr.callCount() != 0 {
		t.Errorf("送信回数 = %d, want 0 (終了済みの ctx で課金を送らない)", tr.callCount())
	}
}

// 14. 受け取ったレスポンスのボディは、再試行する分も含めてすべて閉じる。
func TestCharge_ClosesAllResponseBodies(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{
		respStatus(http.StatusServiceUnavailable, serverBody),
		respStatus(http.StatusServiceUnavailable, serverBody),
		respOK(successBody),
	}}
	c := newTestClient(tr, &fakeSleeper{})

	if _, err := c.Charge(context.Background(), ChargeRequest{OrderID: "order-1", AmountJPY: 4980}); err != nil {
		t.Fatalf("3回目で成功するので、エラーは返らないはず: %v", err)
	}
	if tr.handedOutBodies() != 3 {
		t.Fatalf("レスポンス数 = %d, want 3", tr.handedOutBodies())
	}
	if n := tr.unclosedBodies(); n != 0 {
		t.Errorf("閉じていないレスポンスボディが %d 個ある (再試行して捨てる分も閉じること)", n)
	}
}

// 15. 再試行の設定は Client のフィールドを見る。
//
//	NewClient が埋めた既定値だけで組み立てたクライアントで、その値どおりに動くこと。
//	定数を直接読みに行っていると、設定を変えたときに追随しない。
func TestCharge_UsesConfiguredValues(t *testing.T) {
	tr := &fakeTransport{responses: []stubResponse{respStatus(http.StatusInternalServerError, serverBody)}}
	sl := &fakeSleeper{}
	c := NewClient(testBaseURL,
		WithHTTPClient(&http.Client{Transport: tr}),
		WithSleeper(sl),
	)

	if _, err := c.Charge(context.Background(), ChargeRequest{OrderID: "order-1", AmountJPY: 4980}); err == nil {
		t.Fatal("ずっと 500 なのでエラーを返すこと")
	}
	if tr.callCount() != c.MaxAttempts {
		t.Errorf("送信回数 = %d, want %d (Client.MaxAttempts に従うこと)", tr.callCount(), c.MaxAttempts)
	}

	got := sl.durations()
	if len(got) == 0 {
		t.Fatal("待機していない")
	}
	if got[0] != c.BaseDelay {
		t.Errorf("最初の待機 = %v, want %v (Client.BaseDelay に従うこと)", got[0], c.BaseDelay)
	}

	keys := tr.idempotencyKeys()
	if keys[0] == "" {
		t.Fatal("Idempotency-Key が空")
	}
	for i, k := range keys {
		if k != keys[0] {
			t.Fatalf("再試行で冪等キーが変わっている (%d本目 = %q, 1本目 = %q)", i+1, k, keys[0])
		}
	}
}
