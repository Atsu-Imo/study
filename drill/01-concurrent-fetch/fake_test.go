package catalog

import (
	"context"
	"sync"
	"time"
)

// probe は各フェイククライアントに埋め込む共通部品。
// 「呼ばれたか」「ctx がキャンセルされたか」を記録し、delay の間だけ応答を遅らせる。
type probe struct {
	delay time.Duration

	mu        sync.Mutex
	called    bool
	cancelled bool
}

// wait は delay 経過を待つ。待っている間に ctx が終了したらそれを記録して ctx.Err() を返す。
func (p *probe) wait(ctx context.Context) error {
	p.mu.Lock()
	p.called = true
	p.mu.Unlock()

	if p.delay <= 0 {
		// 遅延なしでも、すでに終了済みの ctx で呼ばれたら失敗させる。
		select {
		case <-ctx.Done():
			p.markCancelled()
			return ctx.Err()
		default:
			return nil
		}
	}

	timer := time.NewTimer(p.delay)
	defer timer.Stop()

	select {
	case <-timer.C:
		return nil
	case <-ctx.Done():
		p.markCancelled()
		return ctx.Err()
	}
}

func (p *probe) markCancelled() {
	p.mu.Lock()
	p.cancelled = true
	p.mu.Unlock()
}

func (p *probe) wasCalled() bool {
	p.mu.Lock()
	defer p.mu.Unlock()
	return p.called
}

func (p *probe) wasCancelled() bool {
	p.mu.Lock()
	defer p.mu.Unlock()
	return p.cancelled
}

type fakeProducts struct {
	probe
	result Product
	err    error
}

func (f *fakeProducts) GetProduct(ctx context.Context, id string) (*Product, error) {
	if err := f.wait(ctx); err != nil {
		return nil, err
	}
	if f.err != nil {
		return nil, f.err
	}
	p := f.result
	p.ID = id
	return &p, nil
}

type fakeInventory struct {
	probe
	result Inventory
	err    error
}

func (f *fakeInventory) GetInventory(ctx context.Context, id string) (*Inventory, error) {
	if err := f.wait(ctx); err != nil {
		return nil, err
	}
	if f.err != nil {
		return nil, f.err
	}
	inv := f.result
	inv.ProductID = id
	return &inv, nil
}

type fakeReviews struct {
	probe
	result []Review
	err    error
}

func (f *fakeReviews) ListReviews(ctx context.Context, id string) ([]Review, error) {
	if err := f.wait(ctx); err != nil {
		return nil, err
	}
	if f.err != nil {
		return nil, f.err
	}
	return f.result, nil
}
