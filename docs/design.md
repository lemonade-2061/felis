# felis設計(メモ)

## 概要
NixOSベースのWaylandコンポジター(GUIより？)
EndeavorOSみたいなポジションのやつ

## コンポーネント

### コンポジター
- 言語: Rust
- レイアウト: タイルとフローティングで切り替える。
- あとからでも切り替えできるようにする。

### バー
- Zigを使う予定(変わるかもしれない)
- 自作

### ランチャー
- rofiとUbuntuのアプリの欄みたいなやつを自作(共存させる)

### アプリ管理
- flatpak + GUI
- Nixでシステム管理

### デフォルトターミナル
- kitty

### CLIツール
- Zig または Rust（未確定。旧方針はGo）

## ベース
- NixOS上のDE
- Flatpakでアプリ管理
- Nixで環境再現性を担保

## 問題
- NixとFlatpakの共存
- NvidiaGPU,IntelGPUの最適化(もってないから)

---
| コンポーネント | 言語 |
| --- | --- |
| コンポジター | Rust |
| バー | Zig(予定) |
| CLIツール | Zig or Rust（未確定） |
| ランチャーGUI | TS(AGS) |
| 環境管理 | Nix |
