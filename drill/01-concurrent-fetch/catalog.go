// Package catalog は商品詳細ページの組み立てを担当する。
//
// 商品情報・在庫・レビューはそれぞれ別のバックエンドサービスが持っており、
// ページを1枚返すために3本の呼び出しが必要になる。
package catalog

import (
	"context"
	"errors"
	"time"
)

// Product は商品マスタの1件。商品サービスが持つ。
type Product struct {
	ID    string
	Name  string
	Price int
}

// Inventory は在庫サービスが返す在庫状況。
type Inventory struct {
	ProductID string
	Stock     int
}

// Review はレビューサービスが返すレビュー1件。
type Review struct {
	Author string
	Score  int
}

// ProductClient は商品マスタを引くクライアント。
type ProductClient interface {
	GetProduct(ctx context.Context, id string) (*Product, error)
}

// InventoryClient は在庫を引くクライアント。
type InventoryClient interface {
	GetInventory(ctx context.Context, id string) (*Inventory, error)
}

// ReviewClient はレビュー一覧を引くクライアント。
type ReviewClient interface {
	ListReviews(ctx context.Context, id string) ([]Review, error)
}

// Degraded スライスに入る項目名。
const (
	DegradedInventory = "inventory"
	DegradedReviews   = "reviews"
)

// ErrProductUnavailable は必須データである商品情報を取得できなかったことを表す。
//
// 呼び出し元はこれを見て 404 / 503 の切り分けや、上位のリトライ判断を行う。
var ErrProductUnavailable = errors.New("product unavailable")

// ProductPage は組み立て済みの商品詳細ページ。
type ProductPage struct {
	Product Product
	Stock   int
	Reviews []Review

	// Degraded は取得に失敗して欠けている任意項目の名前。
	// ページ自体は返せるが一部が欠損していることを呼び出し元に伝える。
	// 昇順ソート済みで、全項目が取得できた場合は空。
	Degraded []string
}

// Service は3つのバックエンドから商品詳細ページを組み立てる。
type Service struct {
	Products  ProductClient
	Inventory InventoryClient
	Reviews   ReviewClient

	// Timeout はページ組み立て1回にかけてよい時間の上限。
	// 0 の場合は上限を設けず、呼び出し元の ctx にのみ従う。
	Timeout time.Duration
}
