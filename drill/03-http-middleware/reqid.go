package httpapi

import (
	"context"
	"crypto/rand"
	"encoding/hex"
)

// RequestIDHeader はリクエストIDを載せるヘッダ名。
//
// ロードバランサが付けてくることもあれば、付いていないこともある。
// 付いていればそれを引き継ぎ、なければこのサービスで発行する。
const RequestIDHeader = "X-Request-Id"

// requestIDKey は ctx のキー。他パッケージのキーと衝突しないよう独自の型にする。
type requestIDKey struct{}

// ContextWithRequestID は ctx にリクエストIDを載せる。
func ContextWithRequestID(ctx context.Context, id string) context.Context {
	return context.WithValue(ctx, requestIDKey{}, id)
}

// RequestIDFrom は ctx からリクエストIDを取り出す。載っていなければ空文字。
//
// ハンドラや下層のサービスはこれを使ってログを1リクエストに紐づける。
func RequestIDFrom(ctx context.Context) string {
	id, _ := ctx.Value(requestIDKey{}).(string)
	return id
}

// NewRequestID は新しいリクエストIDを発行する。
func NewRequestID() string {
	var b [8]byte
	if _, err := rand.Read(b[:]); err != nil {
		panic(err)
	}
	return hex.EncodeToString(b[:])
}
