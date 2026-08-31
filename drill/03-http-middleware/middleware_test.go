package httpapi

import (
	"encoding/json"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"slices"
	"strings"
	"testing"
)

// okHandler は 200 と body を返すだけのハンドラ。
func okHandler(body string) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(body))
	})
}

// panicHandler は panic するハンドラ。
func panicHandler(v any) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		panic(v)
	})
}

// 1. Chain は先頭の引数が最も外側になる。
func TestChain_AppliesOutermostFirst(t *testing.T) {
	var calls []string

	mark := func(name string) Middleware {
		return func(next http.Handler) http.Handler {
			return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				calls = append(calls, "in:"+name)
				next.ServeHTTP(w, r)
				calls = append(calls, "out:"+name)
			})
		}
	}

	h := Chain(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		calls = append(calls, "handler")
	}), mark("a"), mark("b"))

	h.ServeHTTP(httptest.NewRecorder(), httptest.NewRequest(http.MethodGet, "/", nil))

	want := []string{"in:a", "in:b", "handler", "out:b", "out:a"}
	if !slices.Equal(calls, want) {
		t.Errorf("呼ばれた順 = %v, want %v (先頭の引数が最外側)", calls, want)
	}
}

// 2. ミドルウェアが1つもなければ、渡したハンドラがそのまま動く。
func TestChain_NoMiddlewares(t *testing.T) {
	rec := httptest.NewRecorder()
	Chain(okHandler("bare")).ServeHTTP(rec, httptest.NewRequest(http.MethodGet, "/", nil))

	if got := rec.Body.String(); got != "bare" {
		t.Errorf("body = %q, want %q", got, "bare")
	}
}

// 3. リクエストIDが付いていなければ発行し、ctx とレスポンスヘッダに載せる。
func TestRequestID_GeneratesWhenAbsent(t *testing.T) {
	var seen string
	h := RequestID(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		seen = RequestIDFrom(r.Context())
		w.WriteHeader(http.StatusOK)
	}))

	rec := httptest.NewRecorder()
	h.ServeHTTP(rec, httptest.NewRequest(http.MethodGet, "/orders/ord-1", nil))

	if seen == "" {
		t.Fatal("ハンドラの ctx からリクエストIDが取れない")
	}
	if got := rec.Header().Get(RequestIDHeader); got != seen {
		t.Errorf("レスポンスヘッダ %s = %q, want %q (ctx と同じ値)", RequestIDHeader, got, seen)
	}
}

// 4. リクエストIDが付いていれば、それを引き継ぐ。
func TestRequestID_UsesIncomingHeader(t *testing.T) {
	var seen string
	h := RequestID(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		seen = RequestIDFrom(r.Context())
		w.WriteHeader(http.StatusOK)
	}))

	req := httptest.NewRequest(http.MethodGet, "/orders/ord-1", nil)
	req.Header.Set(RequestIDHeader, "lb-generated-42")

	rec := httptest.NewRecorder()
	h.ServeHTTP(rec, req)

	if seen != "lb-generated-42" {
		t.Errorf("ctx のリクエストID = %q, want %q (受け取った値を引き継ぐこと)", seen, "lb-generated-42")
	}
	if got := rec.Header().Get(RequestIDHeader); got != "lb-generated-42" {
		t.Errorf("レスポンスヘッダ %s = %q, want %q", RequestIDHeader, got, "lb-generated-42")
	}
}

// 5. 発行するリクエストIDはリクエストごとに異なる。
func TestRequestID_UniquePerRequest(t *testing.T) {
	h := RequestID(okHandler(""))

	first := httptest.NewRecorder()
	h.ServeHTTP(first, httptest.NewRequest(http.MethodGet, "/", nil))
	second := httptest.NewRecorder()
	h.ServeHTTP(second, httptest.NewRequest(http.MethodGet, "/", nil))

	a := first.Header().Get(RequestIDHeader)
	b := second.Header().Get(RequestIDHeader)
	if a == "" || b == "" {
		t.Fatalf("リクエストIDが空 (1本目=%q 2本目=%q)", a, b)
	}
	if a == b {
		t.Errorf("2本のリクエストで同じID (%q)。リクエストごとに発行すること", a)
	}
}

// 6. ResponseObserver は書かれたステータスとバイト数を記録する。
func TestObserver_TracksStatusAndBytes(t *testing.T) {
	rec := httptest.NewRecorder()
	obs := NewResponseObserver(rec)

	if obs.Status() != 0 {
		t.Errorf("書き込み前の Status() = %d, want 0", obs.Status())
	}
	if obs.Written() {
		t.Error("書き込み前の Written() が true")
	}

	obs.WriteHeader(http.StatusNotFound)
	n, err := obs.Write([]byte("not found"))
	if err != nil {
		t.Fatalf("Write が失敗: %v", err)
	}
	if n != len("not found") {
		t.Errorf("Write の戻り値 = %d, want %d (下位の戻り値をそのまま返すこと)", n, len("not found"))
	}

	if obs.Status() != http.StatusNotFound {
		t.Errorf("Status() = %d, want 404", obs.Status())
	}
	if obs.BytesWritten() != len("not found") {
		t.Errorf("BytesWritten() = %d, want %d", obs.BytesWritten(), len("not found"))
	}
	if !obs.Written() {
		t.Error("Written() が false")
	}
	if rec.Code != http.StatusNotFound {
		t.Errorf("下位に伝わったステータス = %d, want 404", rec.Code)
	}
	if got := rec.Body.String(); got != "not found" {
		t.Errorf("下位に伝わった本文 = %q, want %q", got, "not found")
	}
}

// 7. WriteHeader を呼ばずに Write されたら、ステータスは 200 とみなす。
func TestObserver_DefaultsToStatus200(t *testing.T) {
	rec := httptest.NewRecorder()
	obs := NewResponseObserver(rec)

	if _, err := obs.Write([]byte("hello")); err != nil {
		t.Fatalf("Write が失敗: %v", err)
	}

	if obs.Status() != http.StatusOK {
		t.Errorf("Status() = %d, want 200 (WriteHeader なしの Write は暗黙の 200)", obs.Status())
	}
	if !obs.Written() {
		t.Error("Written() が false")
	}
}

// 8. WriteHeader が2回呼ばれても、最初のステータスが残る。
func TestObserver_IgnoresSecondWriteHeader(t *testing.T) {
	rec := httptest.NewRecorder()
	obs := NewResponseObserver(rec)

	obs.WriteHeader(http.StatusCreated)
	obs.WriteHeader(http.StatusInternalServerError)

	if obs.Status() != http.StatusCreated {
		t.Errorf("Status() = %d, want 201 (2回目以降の WriteHeader は無視)", obs.Status())
	}
	if rec.Code != http.StatusCreated {
		t.Errorf("下位に伝わったステータス = %d, want 201", rec.Code)
	}
}

// 9. 包んだあとも http.ResponseController 経由の Flush が下位に届く。
func TestObserver_SupportsResponseController(t *testing.T) {
	rec := httptest.NewRecorder()
	obs := NewResponseObserver(rec)

	if err := http.NewResponseController(obs).Flush(); err != nil {
		t.Fatalf("Flush が届いていない: %v (包むと下位の機能が隠れる)", err)
	}
	if !rec.Flushed {
		t.Error("下位の ResponseWriter が Flush されていない")
	}
}

// 10. アクセスログはリクエスト1本につき1行。
func TestAccessLog_LogsRequestSummary(t *testing.T) {
	capture := newLogCapture()

	h := AccessLog(capture.logger())(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusNotFound)
		_, _ = w.Write([]byte(`{"code":"x"}`))
	}))

	req := httptest.NewRequest(http.MethodGet, "/orders/ord-1", nil)
	req = req.WithContext(ContextWithRequestID(req.Context(), "req-fixed"))
	h.ServeHTTP(httptest.NewRecorder(), req)

	if n := capture.count("request"); n != 1 {
		t.Fatalf(`message="request" のログが %d 件。リクエスト1本につき1行にすること`, n)
	}

	rec := capture.find(t, "request")
	if rec.Level != slog.LevelInfo {
		t.Errorf("ログレベル = %v, want INFO", rec.Level)
	}
	if got := attrString(t, rec, "method"); got != http.MethodGet {
		t.Errorf("method = %q, want %q", got, http.MethodGet)
	}
	if got := attrString(t, rec, "path"); got != "/orders/ord-1" {
		t.Errorf("path = %q, want %q", got, "/orders/ord-1")
	}
	if got := attrInt(t, rec, "status"); got != http.StatusNotFound {
		t.Errorf("status = %d, want 404", got)
	}
	if got := attrInt(t, rec, "bytes"); got != len(`{"code":"x"}`) {
		t.Errorf("bytes = %d, want %d", got, len(`{"code":"x"}`))
	}
	if got := attrString(t, rec, "request_id"); got != "req-fixed" {
		t.Errorf("request_id = %q, want %q", got, "req-fixed")
	}
	if _, ok := rec.Attrs["duration_ms"]; !ok {
		t.Errorf("属性 duration_ms がない (属性: %v)", rec.Attrs)
	}
}

// 11. ハンドラが WriteHeader を呼ばなくても、ログのステータスは 200。
func TestAccessLog_DefaultsStatusTo200(t *testing.T) {
	capture := newLogCapture()

	h := AccessLog(capture.logger())(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write([]byte("ok"))
	}))
	h.ServeHTTP(httptest.NewRecorder(), httptest.NewRequest(http.MethodGet, "/healthz", nil))

	if got := attrInt(t, capture.find(t, "request"), "status"); got != http.StatusOK {
		t.Errorf("status = %d, want 200", got)
	}
}

// 12. panic を受け止めて 500 を返す。
func TestRecover_Returns500(t *testing.T) {
	capture := newLogCapture()
	h := Recover(capture.logger())(panicHandler("boom"))

	rec := httptest.NewRecorder()
	h.ServeHTTP(rec, httptest.NewRequest(http.MethodGet, "/orders/ord-1", nil))

	if rec.Code != http.StatusInternalServerError {
		t.Errorf("ステータス = %d, want 500", rec.Code)
	}
	if ct := rec.Header().Get("Content-Type"); !strings.HasPrefix(ct, "application/json") {
		t.Errorf("Content-Type = %q, want application/json", ct)
	}

	var body errorBody
	if err := json.Unmarshal(rec.Body.Bytes(), &body); err != nil {
		t.Fatalf("本文が JSON でない (%q): %v", rec.Body.String(), err)
	}
	if body.Code != "internal_error" {
		t.Errorf("本文の code = %q, want %q", body.Code, "internal_error")
	}
	if strings.Contains(rec.Body.String(), "boom") {
		t.Error("panic の内容がクライアントに漏れている")
	}
}

// 13. panic は原因とスタックを添えて ERROR で記録する。
func TestRecover_LogsPanicWithStack(t *testing.T) {
	capture := newLogCapture()
	h := Recover(capture.logger())(panicHandler("boom"))

	req := httptest.NewRequest(http.MethodGet, "/orders/ord-1", nil)
	req = req.WithContext(ContextWithRequestID(req.Context(), "req-fixed"))
	h.ServeHTTP(httptest.NewRecorder(), req)

	rec := capture.find(t, "panic")
	if rec.Level != slog.LevelError {
		t.Errorf("ログレベル = %v, want ERROR", rec.Level)
	}
	if got := attrString(t, rec, "panic"); !strings.Contains(got, "boom") {
		t.Errorf("属性 panic = %q, panic した値を含めること", got)
	}
	if got := attrString(t, rec, "stack"); !strings.Contains(got, "goroutine") {
		t.Errorf("属性 stack = %q, スタックトレースを含めること", got)
	}
	if got := attrString(t, rec, "request_id"); got != "req-fixed" {
		t.Errorf("request_id = %q, want %q", got, "req-fixed")
	}
}

// 14. すでに本文を書き始めていたら、レスポンスに手を加えない。
func TestRecover_DoesNotTouchAlreadyWrittenResponse(t *testing.T) {
	capture := newLogCapture()

	handler := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{"partial":true}`))
		panic("途中で落ちた")
	})
	h := Chain(handler, AccessLog(capture.logger()), Recover(capture.logger()))

	rec := httptest.NewRecorder()
	h.ServeHTTP(rec, httptest.NewRequest(http.MethodGet, "/orders/ord-1", nil))

	if rec.Code != http.StatusOK {
		t.Errorf("ステータス = %d, want 200 (書き始めた後はもう変えられない)", rec.Code)
	}
	if got := rec.Body.String(); got != `{"partial":true}` {
		t.Errorf("本文 = %q, want %q (壊れた本文に追記しないこと)", got, `{"partial":true}`)
	}
	if got := attrInt(t, capture.find(t, "request"), "status"); got != http.StatusOK {
		t.Errorf("アクセスログの status = %d, want 200", got)
	}
	capture.find(t, "panic") // panic 自体は記録すること
}

// 15. http.ErrAbortHandler は握り潰さずに投げ直す。
func TestRecover_RepanicsAbortHandler(t *testing.T) {
	capture := newLogCapture()
	h := Recover(capture.logger())(panicHandler(http.ErrAbortHandler))

	defer func() {
		if got := recover(); got != http.ErrAbortHandler {
			t.Errorf("recover した値 = %v, want http.ErrAbortHandler (net/http に処理させること)", got)
		}
	}()

	h.ServeHTTP(httptest.NewRecorder(), httptest.NewRequest(http.MethodGet, "/orders/ord-1", nil))
}

// 16. 組み立てた Server では、panic したリクエストもアクセスログに 500 で残る。
//
//	panic ログとアクセスログに同じリクエストIDが入っていること。
func TestServer_PanicIsLoggedWithStatusAndRequestID(t *testing.T) {
	capture := newLogCapture()
	srv := &Server{
		// Customer が入っていない移行データ。ハンドラが nil 参照で落ちる。
		Orders: NewMemoryStore(&Order{ID: "ord-broken", Status: "paid", Total: 1200}),
		Logger: capture.logger(),
	}

	rec := httptest.NewRecorder()
	srv.Handler().ServeHTTP(rec, httptest.NewRequest(http.MethodGet, "/orders/ord-broken", nil))

	if rec.Code != http.StatusInternalServerError {
		t.Fatalf("ステータス = %d, want 500", rec.Code)
	}

	access := capture.find(t, "request")
	if got := attrInt(t, access, "status"); got != http.StatusInternalServerError {
		t.Errorf("アクセスログの status = %d, want 500 (panic の処理より外側にあること)", got)
	}

	id := rec.Header().Get(RequestIDHeader)
	if id == "" {
		t.Fatal("レスポンスにリクエストIDがない")
	}
	if got := attrString(t, access, "request_id"); got != id {
		t.Errorf("アクセスログの request_id = %q, want %q", got, id)
	}
	if got := attrString(t, capture.find(t, "panic"), "request_id"); got != id {
		t.Errorf("panic ログの request_id = %q, want %q (同じリクエストとして追えること)", got, id)
	}
}

// 17. ミドルウェアを通しても、ストリーミングのハンドラが Flush できる。
func TestServer_StreamingStillFlushes(t *testing.T) {
	capture := newLogCapture()
	srv := &Server{
		Orders: NewMemoryStore(&Order{ID: "ord-1", Status: "paid", Total: 980, Customer: &Customer{ID: "cus-1", Name: "伊藤"}}),
		Logger: capture.logger(),
	}

	rec := httptest.NewRecorder()
	srv.Handler().ServeHTTP(rec, httptest.NewRequest(http.MethodGet, "/orders/ord-1/events", nil))

	if rec.Code != http.StatusOK {
		t.Fatalf("ステータス = %d, want 200", rec.Code)
	}
	if got := strings.Count(rec.Body.String(), "data:"); got != 3 {
		t.Errorf("届いたイベント = %d 件, want 3 (Flush できずに打ち切られていないか): %q", got, rec.Body.String())
	}
}
