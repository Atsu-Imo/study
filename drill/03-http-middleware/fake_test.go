package httpapi

import (
	"context"
	"fmt"
	"log/slog"
	"sync"
	"testing"
)

// logRecord は捕まえたログ1件。
type logRecord struct {
	Level   slog.Level
	Message string
	Attrs   map[string]any
}

// logStore は捕まえたログの置き場。WithAttrs で分岐したハンドラ間で共有する。
type logStore struct {
	mu      sync.Mutex
	records []logRecord
}

// logCapture はテスト用の slog.Handler。出力せずにレコードを溜める。
type logCapture struct {
	store *logStore
	base  []slog.Attr
}

func newLogCapture() *logCapture {
	return &logCapture{store: &logStore{}}
}

func (c *logCapture) logger() *slog.Logger {
	return slog.New(c)
}

func (c *logCapture) Enabled(context.Context, slog.Level) bool { return true }

func (c *logCapture) Handle(_ context.Context, r slog.Record) error {
	rec := logRecord{Level: r.Level, Message: r.Message, Attrs: make(map[string]any)}
	for _, a := range c.base {
		rec.Attrs[a.Key] = a.Value.Any()
	}
	r.Attrs(func(a slog.Attr) bool {
		rec.Attrs[a.Key] = a.Value.Any()
		return true
	})

	c.store.mu.Lock()
	defer c.store.mu.Unlock()
	c.store.records = append(c.store.records, rec)
	return nil
}

func (c *logCapture) WithAttrs(attrs []slog.Attr) slog.Handler {
	merged := make([]slog.Attr, 0, len(c.base)+len(attrs))
	merged = append(merged, c.base...)
	merged = append(merged, attrs...)
	return &logCapture{store: c.store, base: merged}
}

func (c *logCapture) WithGroup(string) slog.Handler { return c }

// records は捕まえたログを出た順に返す。
func (c *logCapture) records() []logRecord {
	c.store.mu.Lock()
	defer c.store.mu.Unlock()
	return append([]logRecord(nil), c.store.records...)
}

// find は message が msg のログを1件返す。見つからなければテストを落とす。
func (c *logCapture) find(t *testing.T, msg string) logRecord {
	t.Helper()

	var seen []string
	for _, rec := range c.records() {
		if rec.Message == msg {
			return rec
		}
		seen = append(seen, rec.Message)
	}
	t.Fatalf("message=%q のログが出ていない (出たログ: %v)", msg, seen)
	return logRecord{}
}

// count は message が msg のログの件数。
func (c *logCapture) count(msg string) int {
	n := 0
	for _, rec := range c.records() {
		if rec.Message == msg {
			n++
		}
	}
	return n
}

// attrInt はログ属性を数値として読む。slog は整数を int64 で持つことがある。
func attrInt(t *testing.T, rec logRecord, key string) int {
	t.Helper()

	v, ok := rec.Attrs[key]
	if !ok {
		t.Fatalf("ログに属性 %q がない (属性: %v)", key, rec.Attrs)
	}
	switch n := v.(type) {
	case int:
		return n
	case int64:
		return int(n)
	case uint64:
		return int(n)
	case float64:
		return int(n)
	}
	t.Fatalf("属性 %q が数値でない: %T (%v)", key, v, v)
	return 0
}

// attrString はログ属性を文字列として読む。
func attrString(t *testing.T, rec logRecord, key string) string {
	t.Helper()

	v, ok := rec.Attrs[key]
	if !ok {
		t.Fatalf("ログに属性 %q がない (属性: %v)", key, rec.Attrs)
	}
	return fmt.Sprint(v)
}
