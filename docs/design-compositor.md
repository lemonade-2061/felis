# felis-compositor

## 概要

wayland用のフローティング型compositor

## 使用技術

| 用途 | 技術 |
| --- | --- |
| 言語 | Rust |
| フレームワーク | smithay |
| イベントループ | calloop |
| 入力処理 | libinput |
| キーマップ | xkdcommon |
| レンダリング | OpenGL or Valkan |
| GPU制御 | DRM/KMS + GBM |
| 開発用バックエンド | winit |

## 機能

- fvwmのwayland版
- デフォルト:フローティング(タイル型も可)
