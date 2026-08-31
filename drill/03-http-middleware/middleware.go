package httpapi

import (
	"log/slog"
	"net/http"
	"time"
)

// Middleware は http.Handler を包んで別の http.Handler にする。
type Middleware func(http.Handler) http.Handler

// Chain は h に mws を適用して1つの http.Handler にする。
//
// 満たすべき要件は README.md を参照。
func Chain(h http.Handler, mws ...Middleware) http.Handler {
	for i := len(mws) - 1; i >= 0; i-- {
		h = mws[i](h)
	}
	return h
}

// RequestID はリクエストIDを引き継ぐか発行し、ctx とレスポンスヘッダに載せる。
//
// 満たすべき要件は README.md を参照。
func RequestID(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// リクエストからX-Request-Idを取得
		rid := r.Header.Get(RequestIDHeader)
		if rid == "" {
			rid = NewRequestID()
		}
		// レスポンスヘッダーにセット
		w.Header().Set(RequestIDHeader, rid)
		// contextにもセット
		nctx := ContextWithRequestID(r.Context(), rid)
		nr := r.WithContext(nctx)
		next.ServeHTTP(w, nr)
	})
}

// AccessLog はリクエスト1本につき1行のログを出す。
//
// 満たすべき要件は README.md を参照。
func AccessLog(logger *slog.Logger) Middleware {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			obs := newResponseObserver(w)
			start := time.Now()
			next.ServeHTTP(obs, r)
			logger.Info("request",
				slog.String("method", r.Method),
				slog.String("path", r.URL.Path),
				slog.Int("status", obs.Status()),
				slog.Int("bytes", obs.BytesWritten()),
				slog.String("request_id", RequestIDFrom(r.Context())),
				slog.Duration("duration_ms", time.Since(start)),
			)
		})
	}
}

// Recover はハンドラの panic を受け止め、プロセスを落とさずにエラーを返す。
//
// 満たすべき要件は README.md を参照。
func Recover(logger *slog.Logger) Middleware {
	panic("not implemented")
}

// NewResponseObserver は w を包んで、書き込みの結果を観測できるようにする。
//
// 満たすべき要件は README.md を参照。
func NewResponseObserver(w http.ResponseWriter) ResponseObserver {
	return newResponseObserver(w)
}

type responseObserver struct {
	http.ResponseWriter
	status  int
	bytes   int
	written bool
}

func newResponseObserver(w http.ResponseWriter) *responseObserver {
	return &responseObserver{ResponseWriter: w}
}

func (ro *responseObserver) Status() int {
	if !ro.written {
		return 0
	}
	if ro.written && ro.status == 0 {
		return http.StatusOK
	}
	return ro.status
}

func (ro *responseObserver) BytesWritten() int {
	return ro.bytes
}

func (ro *responseObserver) Written() bool {
	return ro.written
}

func (ro *responseObserver) WriteHeader(code int) {
	if ro.written {
		return
	}
	ro.status = code
	ro.ResponseWriter.WriteHeader(code)
	ro.written = true
}

func (ro *responseObserver) Write(bytes []byte) (int, error) {
	i, e := ro.ResponseWriter.Write(bytes)
	ro.bytes = ro.bytes + i
	ro.written = true
	return i, e
}

func (ro *responseObserver) Unwrap() http.ResponseWriter {
	return ro.ResponseWriter
}
