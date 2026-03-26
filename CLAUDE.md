# study リポジトリ — Claude向け指示書

## プロジェクト概要

このリポジトリはプログラミング言語やクラウドサービスの学習用。各トピックはサブディレクトリとして管理する。

## ディレクトリ構成

- `rust/` — Rust学習 (Go経験者向け)。詳細は `rust/CLAUDE.md` を参照
- `coq/` — Coq学習 (ソフトウェア検証)。詳細は `coq/CLAUDE.md` を参照
- `aws/` — AWS学習 (未経験・理解不足のサービス)。詳細は `aws/CLAUDE.md` を参照

## 開発環境

- `rust/` — devcontainer で環境構築 (`.devcontainer/` 配置)
- `coq/` — WSL2 に Rocq Platform を直接インストール。CoqIDE を使用
