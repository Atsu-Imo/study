package httpapi

import "net/http"

// ResponseObserver は書き込みの結果を観測できる http.ResponseWriter。
//
// アクセスログはハンドラが返したステータスコードと本文のバイト数を知る必要があるが、
// http.ResponseWriter にはそれを問い合わせる口がない。そこで一段かぶせて記録する。
//
// panic の処理側も「もう本文を書き始めているか」を知る必要がある。
// 書き始めたあとではステータスコードを差し替えられないため。
type ResponseObserver interface {
	http.ResponseWriter

	// Status は書き込まれたステータスコード。まだ何も書かれていなければ 0。
	Status() int

	// BytesWritten は本文として書き込まれたバイト数の累計。
	BytesWritten() int

	// Written はヘッダまたは本文が書き込まれたかどうか。
	Written() bool
}
