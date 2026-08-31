package catalog

import (
	"context"
	"fmt"
	"sort"
	"sync"
)

// BuildProductPage は id の商品詳細ページを組み立てて返す。
//
// 満たすべき要件は README.md を参照。
func (s *Service) BuildProductPage(ctx context.Context, id string) (*ProductPage, error) {
	// 10. 呼び出し元の ctx がキャンセルされたら、それを尊重して速やかに返る
	var cancel context.CancelFunc
	if s.Timeout > 0 {
		// 11. Service.Timeout が 0 より大きければ、それを組み立て全体の上限とする
		ctx, cancel = context.WithTimeout(ctx, s.Timeout)
	} else {
		// 6. 商品情報の失敗が確定した時点で、残りの取得をキャンセルする。 不要になった結果を待たない
		ctx, cancel = context.WithCancel(ctx)
	}
	defer cancel()
	// 12. 期限切れの時点で必須データが取れていなければエラー （errors.Is(err, context.DeadlineExceeded) が真になること）
	// 13. 期限切れの時点で必須データが取れていれば、残りを縮退させてページを返す

	// 気になる
	// * degradedの扱いがダサい
	// * wg2つはどうなんだろう
	// * 定義が多くないか？
	// * 全体的に愚直な実装が多すぎて無駄に見える

	// 1. 3本の取得は並行に実行する。逐次だと全体のレイテンシが3倍になる
	// 2. 関数から戻ったあと、goroutine を残さない
	wg := sync.WaitGroup{}
	wg.Add(3)

	var product *Product
	var inventory *Inventory
	var reviews []Review

	var productErr, inventoryErr, reviewsErr error

	go func() {
		defer wg.Done()
		product, productErr = s.Products.GetProduct(ctx, id)
		if productErr != nil {
			cancel()
		}
	}()

	go func() {
		defer wg.Done()
		inventory, inventoryErr = s.Inventory.GetInventory(ctx, id)
	}()

	go func() {
		defer wg.Done()
		reviews, reviewsErr = s.Reviews.ListReviews(ctx, id)
	}()

	wg.Wait()

	// 3. 商品情報が取得できなかったら、ページを返さずエラーを返す
	if productErr != nil {
		// 4. そのエラーは errors.Is(err, ErrProductUnavailable) で判別できること
		// 5. 同時に、元の原因も errors.Is で辿れること
		return nil, fmt.Errorf("%w: %w", ErrProductUnavailable, productErr)
	}

	return buildPage(product, inventory, inventoryErr, reviews, reviewsErr), nil
}

func buildPage(p *Product, i *Inventory, ie error, r []Review, re error) *ProductPage {
	var rProduct *Product
	var rStock int
	var rReviews []Review
	// 8. 失敗した項目の値はゼロ値のままにし、項目名を ProductPage.Degraded に入れる （定数 DegradedInventory / DegradedReviews を使う）
	var degraded []string

	if p != nil {
		rProduct = p
	} else {
		rProduct = &Product{}
	}
	// 7. 在庫・レビューの取得が失敗しても、エラーにせずページを返す
	if ie != nil {
		rStock = 0
		degraded = append(degraded, DegradedInventory)
	} else if i == nil {
		rStock = 0
	} else {
		rStock = i.Stock
	}
	if re != nil {
		rReviews = []Review{}
		degraded = append(degraded, DegradedReviews)
	} else {
		rReviews = r
	}
	sort.Strings(degraded)
	return &ProductPage{
		Product:  *rProduct,
		Stock:    rStock,
		Reviews:  rReviews,
		Degraded: degraded,
	}
}
