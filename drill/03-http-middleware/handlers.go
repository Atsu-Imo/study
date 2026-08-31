package httpapi

import (
	"encoding/json"
	"fmt"
	"net/http"
)

// Order は注文1件。
type Order struct {
	ID       string
	Status   string
	Total    int
	Customer *Customer
}

// Customer は注文者。
type Customer struct {
	ID   string
	Name string
}

// OrderStore は注文を引く。
type OrderStore interface {
	Get(id string) (*Order, bool)
}

// MemoryStore は開発・テスト用のインメモリ実装。
type MemoryStore struct {
	orders map[string]*Order
}

// NewMemoryStore は与えられた注文を持つストアを作る。
func NewMemoryStore(orders ...*Order) *MemoryStore {
	m := &MemoryStore{orders: make(map[string]*Order, len(orders))}
	for _, o := range orders {
		m.orders[o.ID] = o
	}
	return m
}

func (s *MemoryStore) Get(id string) (*Order, bool) {
	o, ok := s.orders[id]
	return o, ok
}

// orderResponse は注文取得のレスポンス。
type orderResponse struct {
	ID           string `json:"id"`
	Status       string `json:"status"`
	Total        int    `json:"total"`
	CustomerName string `json:"customer_name"`
}

// errorBody はエラーレスポンスの本文。
type errorBody struct {
	Code string `json:"code"`
}

func (s *Server) handleHealthz(w http.ResponseWriter, r *http.Request) {
	w.WriteHeader(http.StatusOK)
	fmt.Fprintln(w, "ok")
}

func (s *Server) handleGetOrder(w http.ResponseWriter, r *http.Request) {
	order, ok := s.Orders.Get(r.PathValue("id"))
	if !ok {
		writeJSON(w, http.StatusNotFound, errorBody{Code: "order_not_found"})
		return
	}

	// 旧システムから移行した一部の注文には Customer が入っていない。
	// 移行バッチの修正待ちで、それまではここで nil 参照になることがある。
	writeJSON(w, http.StatusOK, orderResponse{
		ID:           order.ID,
		Status:       order.Status,
		Total:        order.Total,
		CustomerName: order.Customer.Name,
	})
}

// handleOrderEvents は注文の状態変化を Server-Sent Events で流す。
func (s *Server) handleOrderEvents(w http.ResponseWriter, r *http.Request) {
	if _, ok := s.Orders.Get(r.PathValue("id")); !ok {
		writeJSON(w, http.StatusNotFound, errorBody{Code: "order_not_found"})
		return
	}

	w.Header().Set("Content-Type", "text/event-stream")
	w.WriteHeader(http.StatusOK)

	// 1イベントごとに送り出す。バッファに溜めたままだと購読側に届かない。
	rc := http.NewResponseController(w)
	for i := range 3 {
		fmt.Fprintf(w, "data: {\"seq\":%d}\n\n", i)
		if err := rc.Flush(); err != nil {
			// 流せないなら続けても意味がないので打ち切る。
			return
		}
	}
}

// writeJSON はステータスコードと JSON 本文を書く。
func writeJSON(w http.ResponseWriter, status int, body any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(body)
}
