// Package httpapi は注文管理サービスの HTTP 層。
//
// ハンドラには業務のことだけを書く。リクエストIDの発行、アクセスログ、
// panic の処理といった全リクエストに共通する処理はハンドラに書かず、
// ミドルウェアとして1箇所に置く。
package httpapi

import (
	"log/slog"
	"net/http"
)

// Server は HTTP 層の依存をまとめる。
type Server struct {
	Orders OrderStore
	Logger *slog.Logger
}

// Handler はルーティングとミドルウェアを組み立てて1つの http.Handler にする。
//
// 呼び出し側 (cmd/orderapi) はこれを http.Server に渡す:
//
//	srv := &http.Server{
//		Addr:              ":8080",
//		Handler:           s.Handler(),
//		ReadHeaderTimeout: 5 * time.Second,
//	}
func (s *Server) Handler() http.Handler {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", s.handleHealthz)
	mux.HandleFunc("GET /orders/{id}", s.handleGetOrder)
	mux.HandleFunc("GET /orders/{id}/events", s.handleOrderEvents)

	// 外側から順に並べる。この順序には意味がある (README の要件20)。
	return Chain(mux,
		RequestID,
		AccessLog(s.Logger),
		Recover(s.Logger),
	)
}
