# felis 設計ドキュメント

## コンセプト

- 複雑さを見せない、邪魔しない、軽いOS
- ターゲット: **非エンジニア**
- 設定は全部GUIで完結（設定ファイルを直接触らせない）
- Windowsっぽい操作感、カスタマイズ性は残す

---

## 技術スタック

| 項目 | 採用技術 | 理由 |
|------|----------|------|
| カーネル | Linux | 触らない、そのまま使う |
| 言語 | Rust | 速度・安全性 |
| GUI | iced 0.14 | Rust製、Wayland対応 |
| ディスプレイ | Wayland メイン + XWayland | モダン + 互換性 |
| コンポジタ | smithay（自作） | 完全なコントロール |
| 環境定義 | Nix Flakes | 再現性 |
| Rustビルド最適化 | crane | ビルドキャッシュ |
| 起動 | systemd-boot | タイムアウト0秒で隠す |

---

## OS構造

```
┌─────────────────────────────────────┐
│           ユーザーランド              │
│                                     │
│  ┌─────────────────────────────┐    │
│  │      felis（自分で作る）      │    │
│  │  felis-compositor           │    │
│  │    ↕ Waylandプロトコル       │    │
│  │  felis-desktop              │    │
│  │  felis-bar                  │    │
│  │  felis-launcher             │    │
│  │  felis-settings             │    │
│  │  felis-files                │    │
│  │  felish                     │    │
│  └─────────────────────────────┘    │
│                                     │
│  ┌──────────┐  ┌──────────────┐     │
│  │ブラウザ等 │  │ 一般アプリ    │     │
│  │(既存)    │  │(Flatpak等)   │     │
│  └──────────┘  └──────────────┘     │
│                                     │
├─────────────────────────────────────┤
│  glibc / libwayland / dbus など      │
├─────────────────────────────────────┤
│           Linuxカーネル               │
└─────────────────────────────────────┘
```

---

## コンポーネント一覧

### 自分で作るもの

| コンポーネント | 役割 | 技術 |
|--------------|------|------|
| felis-compositor | Waylandコンポジタ | smithay |
| felis-desktop | 壁紙・デスクトップアイコン | iced |
| felis-bar | タスクバー・時計・音量 | iced + wlr-layer-shell |
| felis-launcher | アプリ起動ランチャー | iced |
| felis-settings | 設定アプリ全般 | iced |
| felis-files | ファイルマネージャー | iced |
| felis-store | アプリストア | iced |
| felis-greeter | ログイン画面（将来） | iced |
| felish | バックエンドシェル | Rust std |

### 触らないもの

- Linuxカーネル
- glibc
- systemd
- Nix

---

## 起動フロー

```
電源ON
  → BIOS/UEFI
  → systemd-boot（タイムアウト0秒、ユーザーに見せない）
  → Linuxカーネル
  → systemd
  → felis-compositor（smithay）
  → felis-desktop + felis-bar
  → デスクトップ表示
```

### ログイン

- 初期実装: greetd で自動ログイン（画面なし）
- 将来: 複数ユーザーが必要になったら felis-greeter を自作
- SDDMなど既存のログインマネージャーは使わない（デザインが合わない）

---

## felis-settings の機能一覧

| 機能 | 実装方法 |
|------|----------|
| 壁紙設定 | iced |
| スタートアップアプリ管理 | systemd ユーザーサービス |
| システム復元・ロールバック | nixos-rebuild（DBus経由） |
| ネットワーク設定 | NetworkManager（DBus / zbus） |
| 電源・バッテリー設定 | UPower + logind（DBus） |
| サウンド設定 | PipeWire（DBus） |
| ユーザー管理 | useradd / usermod / userdel |
| ファイアウォール | nftables |
| キーボード・入力設定 | fcitx5 |
| アップデート通知 | Nix世代管理 |

---

## Nixのロールバック設計

```
通常: 最新世代で自動起動（ブートローダー非表示）

復元: felis-settings の「システム復元」からGUIで操作
      → nixos-rebuild switch --rollback を裏で呼ぶ
      → ブートローダーを触らなくていい

UI:
  現在のシステム: Generation 5
  ┌─────────────────────────┐
  │ Generation 4  2026-03-10│
  │ Generation 3  2026-03-01│
  └─────────────────────────┘
  [この世代に戻す]
```

---

## ネットワーク

- デーモン: **NetworkManager**
- 通信方法: DBus（zbusクレート）
- 対応: WiFi・有線LAN・将来的にVPN
- UI: felis-settings 内のネットワーク設定画面

---

## 音声

- 音声サーバー: **PipeWire**
- 互換性: PulseAudio・ALSA・JACK互換
- Bluetooth: bluez + PipeWire連携
- UI:
  - felis-bar: 音量アイコン・クイック調整
  - felis-settings: 詳細設定・デバイス切替
- Nixで有効化するだけ、独自実装ほぼ不要

---

## バッテリー・電源管理

```
検出方法:
  /sys/class/power_supply/BAT0 の存在で判定
  より安定した方法として UPower（DBus）も使用

バッテリーあり（ノートPC）:
  felis-bar にバッテリーアイコン・残量表示
  felis-settings に電源設定画面
  20%以下で警告表示（赤色）

バッテリーなし（デスクトップ）:
  バッテリー関連UI非表示

電源管理デーモン: logind（systemd組み込み）
バッテリー情報:   UPower（DBus経由）
```

---

## セキュリティ

| 項目 | 採用技術 | 方針 |
|------|----------|------|
| プロセス制限 | AppArmor | Nixでプロファイル定義、ユーザーは触らない |
| ファイアウォール | nftables | デフォルトON、インバウンドブロック |
| 認証 | PAM + polkit | GUIダイアログでパスワード確認 |
| アプリ隔離 | Flatpak + Bubblewrap | サンドボックス |
| アップデート | Nixの世代管理 | 自動アップデート通知 |

**方針: デフォルトで安全、GUIで必要な設定だけ変更可能**

---

## パッケージ管理

```
システム:       Nix（nixpkgs）
ユーザーアプリ:  Flatpak（Flathub）メイン
               + nix profile（開発者・上級者向け）

UI: felis-store
  ・nixpkgs + Flathub を統合検索
  ・インストール進捗をリアルタイム表示
  ・裏で nix / flatpak コマンドを叩く
  ・ユーザーはコマンドを意識しなくていい

役割分担:
  Nix      → システムツール・felis自身のコンポーネント
  Flatpak  → ブラウザ・動画プレイヤーなど一般アプリ
```

---

## ユーザー管理

```
種類:
  管理者（wheelグループ）→ システム設定変更可
  標準ユーザー          → 自分の設定のみ変更可

実装:
  useradd / usermod / userdel コマンドを裏で叩く
  polkit で管理者権限の認証ダイアログ

機能（felis-settings）:
  ユーザー追加・削除
  権限変更（管理者 ↔ 標準）
  パスワード変更
  自動ログインON/OFF
```

---

## 日本語入力

- IME: **fcitx5-mozc**（辞書強化版）
- 辞書: mozc-ut または NEologd で精度改善
- Wayland統合: text-input-v3 プロトコル
- 将来: 変換精度が問題になればオンライン変換（Google CGI API）も検討

---

## ファイルマネージャー（felis-files）

**方針: 最低限から自作、段階的に拡張**

```
Phase 2（最低限）:
  ✅ フォルダ移動・ファイル一覧表示
  ✅ コピー・移動・削除・リネーム
  ✅ ゴミ箱
  ✅ 右クリックメニュー

Phase 3（拡張）:
  ⬜ サムネイル表示
  ⬜ ドラッグ&ドロップ
  ⬜ アプリとの関連付け
  ⬜ 圧縮・解凍

Phase 4（将来）:
  ⬜ ネットワークドライブ
  ⬜ タグ・検索機能
```

**注意: ファイル操作のバグはデータ消失に直結するためテストを丁寧に書く**

---

## スタートアップ管理

- 実装: systemd ユーザーサービス（~/.config/systemd/user/）
- UI: felis-settings > スタートアップ（アプリ一覧 ON/OFF）
- ユーザーはサービスファイルを直接触らなくていい

---

## felis-studio（クリエイティブスイート）

**OSを先に作る理由:**
- 描画パイプラインを完全にコントロールできる
- クリエイティブ用途向けのカーネル最適化ができる
- Nixで環境を完全に固定できる
- 「felisOS上では最高のパフォーマンス」を保証できる

**対象: Linux全般 + felisOS同梱**

### 開発順序

| 順番 | 名前 | 代替ソフト | 技術 |
|------|------|-----------|------|
| 1 | felis-photo | Lightroom | rawler・palette・image |
| 2 | felis-vector | Illustrator | kurbo・usvg |
| 3 | felis-paint | Photoshop | image・wgpu |
| 4 | felis-cut | Premiere | ffmpeg-next |
| 5 | felis-motion | After Effects | wgpu・自作 |

**共通方針: Rust + iced で統一、UIはシンプルに**

---

## 最適化方針

### カーネルレベル

```nix
boot.kernelParams = [
  "threadirqs"   # IRQスレッド化、描画の安定性向上
];

boot.kernel.sysctl = {
  "vm.swappiness" = 10;          # スワップ抑制
  "vm.nr_hugepages" = 128;       # HugePage（大ファイル処理向上）
};

# CPUスケジューラ: BORE（デスクトップ用途に最適化）
# zram: RAMを仮想的に増やす
zramSwap.enable = true;
zramSwap.memoryPercent = 25;
```

### Rustコードレベル

```toml
[profile.release]
opt-level = 3       # 最大最適化
lto = true          # リンク時最適化
codegen-units = 1   # 最適化精度を上げる
strip = true        # バイナリを小さくする
```

```nix
# ネイティブCPU最適化
environment.variables = {
  RUSTFLAGS = "-C target-cpu=native";
};
```

### コードレベルの最適化手法

| 手法 | 効果 | 難易度 |
|------|------|--------|
| cargo --release | 全体的に速くなる | 簡単 |
| Vec::with_capacity | メモリ確保を減らす | 簡単 |
| rayon（並列処理） | CPUコア数分速くなる | 普通 |
| SIMD | 画像処理が最大8倍速 | 難しい |
| wgpu バッチ処理 | GPU描画が安定する | 難しい |

---

## 開発フェーズ

### Phase 1（今）: 基礎
- [x] icedでウィンドウ表示
- [ ] felis-desktop の基本UI（壁紙・背景色）
- [ ] felis-bar の基本UI（時計・音量）
- [ ] Waylandで動作確認

### Phase 2: コンポーネント単体
- [ ] felis-launcher
- [ ] felis-files（ファイルマネージャー最低限）
- [ ] felis-settings（基本設定）
- [ ] felis-store（アプリストア）
- [ ] felish（コマンド実行できるシェル）

### Phase 3: コンポジタ
- [ ] smithay で felis-compositor を作る
- [ ] anvil のコードを読みながら実装
- [ ] wlr-layer-shell 対応（タスクバー本番化）

### Phase 4: 統合
- [ ] 全コンポーネントを組み合わせる
- [ ] NixでISOイメージを作る
- [ ] 単体で起動するOSになる

### Phase 5: felis-studio
- [ ] felis-photo（RAW現像）
- [ ] felis-vector
- [ ] felis-paint
- [ ] felis-cut
- [ ] felis-motion

### Phase 6: システム周り
- [ ] エラーハンドリングの仕組み
- [ ] ログ保存（~/Documents/logs）
- [ ] カーネル最適化の適用

---

## 保留・未決定事項

- ログイン画面（複数ユーザーが必要になったタイミングで自作）
- XWayland の組み込みタイミング
- felis-compositor に移行するタイミング
- NVIDIA GPU の対応（後回し）
- felis-studio の Windows/Mac 対応（将来検討）

---

## 参考リンク

### felis（OS）
- [iced](https://github.com/iced-rs/iced)
- [smithay](https://github.com/Smithay/smithay)
- [anvil（smithayのサンプル）](https://github.com/Smithay/smithay/tree/master/anvil)
- [COSMIC DE（smithay + icedの実例）](https://github.com/pop-os/cosmic-epoch)
- [iced-layershell](https://github.com/waycrate/exwlshelleventloop)
- [crane（RustのNixビルドキャッシュ）](https://github.com/ipetkov/crane)

### システム
- [NetworkManager DBus API](https://networkmanager.dev/docs/api/latest/)
- [zbus（RustのDBusクレート）](https://github.com/dbus2/zbus)
- [UPower](https://upower.freedesktop.org/)
- [PipeWire](https://pipewire.org/)
- [Flatpak](https://docs.flatpak.org/en/latest/)
- [greetd](https://git.sr.ht/~kennylevinsen/greetd)
- [BORE scheduler](https://github.com/firelzrd/bore-scheduler)

### felis-studio
- [rawler（RAW現像）](https://github.com/dnglab/dnglab)
- [kurbo（ベクター計算）](https://docs.rs/kurbo/latest/kurbo/)
- [ffmpeg-next（動画処理）](https://docs.rs/ffmpeg-next/latest/ffmpeg_next/)
- [rayon（並列処理）](https://docs.rs/rayon/latest/rayon/)
- [Rustパフォーマンスの本](https://nnethercote.github.io/perf-book/)
