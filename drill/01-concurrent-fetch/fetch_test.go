package catalog

import (
	"context"
	"errors"
	"runtime"
	"slices"
	"testing"
	"time"
)

// errBackendDown はバックエンドが落ちている状況を表すテスト用のエラー。
var errBackendDown = errors.New("backend down")

func newTestService() (*Service, *fakeProducts, *fakeInventory, *fakeReviews) {
	products := &fakeProducts{result: Product{Name: "Espresso Machine", Price: 42000}}
	inventory := &fakeInventory{result: Inventory{Stock: 7}}
	reviews := &fakeReviews{result: []Review{{Author: "ito", Score: 5}}}

	svc := &Service{
		Products:  products,
		Inventory: inventory,
		Reviews:   reviews,
	}
	return svc, products, inventory, reviews
}

// waitFor は cond が true になるまで最大 timeout だけ待つ。
func waitFor(timeout time.Duration, cond func() bool) bool {
	deadline := time.Now().Add(timeout)
	for time.Now().Before(deadline) {
		if cond() {
			return true
		}
		time.Sleep(10 * time.Millisecond)
	}
	return cond()
}

// 1. 全部成功したら、すべての項目が埋まったページが返る。
func TestBuildProductPage_AllSucceed(t *testing.T) {
	svc, _, _, _ := newTestService()

	page, err := svc.BuildProductPage(context.Background(), "sku-1")
	if err != nil {
		t.Fatalf("エラーは返らないはず: %v", err)
	}
	if page == nil {
		t.Fatal("ページが nil")
	}
	if page.Product.ID != "sku-1" {
		t.Errorf("Product.ID = %q, want %q", page.Product.ID, "sku-1")
	}
	if page.Product.Name != "Espresso Machine" {
		t.Errorf("Product.Name = %q, want %q", page.Product.Name, "Espresso Machine")
	}
	if page.Stock != 7 {
		t.Errorf("Stock = %d, want 7", page.Stock)
	}
	if len(page.Reviews) != 1 {
		t.Errorf("Reviews の件数 = %d, want 1", len(page.Reviews))
	}
	if len(page.Degraded) != 0 {
		t.Errorf("全部成功したら Degraded は空のはず: %v", page.Degraded)
	}
}

// 2. 3本の取得は並行に走る。逐次実行だと 300ms 以上かかる。
func TestBuildProductPage_RunsConcurrently(t *testing.T) {
	svc, products, inventory, reviews := newTestService()
	products.delay = 100 * time.Millisecond
	inventory.delay = 100 * time.Millisecond
	reviews.delay = 100 * time.Millisecond

	start := time.Now()
	if _, err := svc.BuildProductPage(context.Background(), "sku-1"); err != nil {
		t.Fatalf("エラーは返らないはず: %v", err)
	}
	elapsed := time.Since(start)

	if elapsed > 250*time.Millisecond {
		t.Errorf("逐次実行になっている疑い: %v かかった (並行なら 100ms 程度)", elapsed)
	}
}

// 3. 必須データ (商品情報) が失敗したら全体をエラーにする。
//
//	ErrProductUnavailable で判別できること、かつ元の原因も errors.Is で辿れること。
func TestBuildProductPage_ProductFailure(t *testing.T) {
	svc, products, _, _ := newTestService()
	products.err = errBackendDown

	page, err := svc.BuildProductPage(context.Background(), "sku-1")
	if err == nil {
		t.Fatal("商品情報が取れなければエラーを返すこと")
	}
	if page != nil {
		t.Errorf("エラー時はページを返さないこと: %+v", page)
	}
	if !errors.Is(err, ErrProductUnavailable) {
		t.Errorf("errors.Is(err, ErrProductUnavailable) が false: %v", err)
	}
	if !errors.Is(err, errBackendDown) {
		t.Errorf("原因のエラーが辿れない。wrap すること: %v", err)
	}
}

// 4. 必須データが失敗した時点で、残りの取得をキャンセルして早く返る。
func TestBuildProductPage_ProductFailureCancelsOthers(t *testing.T) {
	svc, products, inventory, reviews := newTestService()
	products.err = errBackendDown
	inventory.delay = 3 * time.Second
	reviews.delay = 3 * time.Second

	start := time.Now()
	if _, err := svc.BuildProductPage(context.Background(), "sku-1"); err == nil {
		t.Fatal("エラーを返すこと")
	}
	elapsed := time.Since(start)

	if elapsed > 500*time.Millisecond {
		t.Errorf("必須データの失敗後、不要になった取得を待ってしまっている: %v", elapsed)
	}
	if !waitFor(time.Second, func() bool { return inventory.wasCancelled() && reviews.wasCancelled() }) {
		t.Errorf("残りの取得がキャンセルされていない (inventory=%v reviews=%v)",
			inventory.wasCancelled(), reviews.wasCancelled())
	}
}

// 5. 任意データ (在庫) が失敗しても、ページ自体は縮退して返す。
func TestBuildProductPage_OptionalFailureDegrades(t *testing.T) {
	svc, _, inventory, _ := newTestService()
	inventory.err = errBackendDown

	page, err := svc.BuildProductPage(context.Background(), "sku-1")
	if err != nil {
		t.Fatalf("任意項目の失敗で全体を落としてはいけない: %v", err)
	}
	if page == nil {
		t.Fatal("ページが nil")
	}
	if page.Product.Name != "Espresso Machine" {
		t.Errorf("取れた項目は埋まっているはず: Product.Name = %q", page.Product.Name)
	}
	if page.Stock != 0 {
		t.Errorf("取れなかった在庫はゼロ値のはず: Stock = %d", page.Stock)
	}
	if len(page.Reviews) != 1 {
		t.Errorf("レビューは取れているはず: 件数 = %d", len(page.Reviews))
	}
	if want := []string{DegradedInventory}; !slices.Equal(page.Degraded, want) {
		t.Errorf("Degraded = %v, want %v", page.Degraded, want)
	}
}

// 6. 任意データが2つとも失敗した場合、Degraded は昇順ソートされている。
func TestBuildProductPage_DegradedIsSorted(t *testing.T) {
	svc, _, inventory, reviews := newTestService()
	inventory.err = errBackendDown
	reviews.err = errBackendDown

	page, err := svc.BuildProductPage(context.Background(), "sku-1")
	if err != nil {
		t.Fatalf("任意項目の失敗で全体を落としてはいけない: %v", err)
	}
	want := []string{DegradedInventory, DegradedReviews}
	if !slices.Equal(page.Degraded, want) {
		t.Errorf("Degraded = %v, want %v (毎回同じ順序で返ること)", page.Degraded, want)
	}
}

// 7. 呼び出し元の ctx がキャンセルされたら、それを尊重して速やかに返る。
func TestBuildProductPage_CallerCancel(t *testing.T) {
	svc, products, inventory, reviews := newTestService()
	products.delay = 3 * time.Second
	inventory.delay = 3 * time.Second
	reviews.delay = 3 * time.Second

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	time.AfterFunc(50*time.Millisecond, cancel)

	start := time.Now()
	_, err := svc.BuildProductPage(ctx, "sku-1")
	elapsed := time.Since(start)

	if err == nil {
		t.Fatal("キャンセルされたらエラーを返すこと")
	}
	if !errors.Is(err, context.Canceled) {
		t.Errorf("errors.Is(err, context.Canceled) が false: %v", err)
	}
	if elapsed > time.Second {
		t.Errorf("キャンセルが伝わっていない: %v かかった", elapsed)
	}
}

// 8. Service.Timeout を超え、かつ必須データも取れていなければタイムアウトエラー。
func TestBuildProductPage_TimeoutBeforeProduct(t *testing.T) {
	svc, products, inventory, reviews := newTestService()
	svc.Timeout = 100 * time.Millisecond
	products.delay = 3 * time.Second
	inventory.delay = 3 * time.Second
	reviews.delay = 3 * time.Second

	start := time.Now()
	_, err := svc.BuildProductPage(context.Background(), "sku-1")
	elapsed := time.Since(start)

	if err == nil {
		t.Fatal("タイムアウトしたらエラーを返すこと")
	}
	if !errors.Is(err, context.DeadlineExceeded) {
		t.Errorf("errors.Is(err, context.DeadlineExceeded) が false: %v", err)
	}
	if elapsed > time.Second {
		t.Errorf("Timeout が効いていない: %v かかった", elapsed)
	}
}

// 9. Timeout を超えても、必須データが揃っていれば縮退したページを返す。
func TestBuildProductPage_TimeoutDegradesWhenProductReady(t *testing.T) {
	svc, products, inventory, reviews := newTestService()
	svc.Timeout = 150 * time.Millisecond
	products.delay = 10 * time.Millisecond
	inventory.delay = 3 * time.Second
	reviews.delay = 3 * time.Second

	page, err := svc.BuildProductPage(context.Background(), "sku-1")
	if err != nil {
		t.Fatalf("必須データが揃っているなら縮退して返すこと: %v", err)
	}
	if page == nil {
		t.Fatal("ページが nil")
	}
	if page.Product.Name != "Espresso Machine" {
		t.Errorf("Product.Name = %q, want %q", page.Product.Name, "Espresso Machine")
	}
	want := []string{DegradedInventory, DegradedReviews}
	if !slices.Equal(page.Degraded, want) {
		t.Errorf("Degraded = %v, want %v", page.Degraded, want)
	}
	if !inventory.wasCalled() || !reviews.wasCalled() {
		t.Error("任意項目も呼ばれているはず")
	}
}

// 10. 戻ったあとに goroutine が残らない。
func TestBuildProductPage_NoGoroutineLeak(t *testing.T) {
	before := runtime.NumGoroutine()

	svc, products, inventory, reviews := newTestService()
	svc.Timeout = 100 * time.Millisecond
	products.delay = 10 * time.Millisecond
	inventory.delay = 5 * time.Second
	reviews.delay = 5 * time.Second

	if _, err := svc.BuildProductPage(context.Background(), "sku-1"); err != nil {
		t.Fatalf("必須データが揃っているなら縮退して返すこと: %v", err)
	}

	if !waitFor(2*time.Second, func() bool { return runtime.NumGoroutine() <= before }) {
		t.Errorf("goroutine が残っている: before=%d after=%d", before, runtime.NumGoroutine())
	}
}
