# レイアウトを値として扱う設計 — undo / 保存 / 復元

> 2026-10-22 のセッションでの議論の記録。Claude との会話をまとめたもので、
> 設計の一次資料は `crates/felis-compositor/src/layout/NOTES.md` 側に置く方針。

## 0. 中心の主張

**felis のレイアウトは「値」である。** 木が値だから、保存・復元・undo・リプレイが
すべて同じ仕組みから出てくる。

これが felis の特異性の候補。ShojiWM が「見た目を宣言的にした」のに対して、
felis は「状態と時間を宣言的にする」という別の軸を取る。
見た目の軸で戦うと劣化版 ShojiWM になるので避ける。

### 言い方

- ✗ 「felis はレイアウト保存できます」 → 新しくない(i3 が2014年からやってる)
- ○ **「felis のレイアウトは値だから、保存・復元が正確で、Python スクリプトが IPC を舐める必要がない」**

デモの文言:

- ✗ 「間違えて閉じても戻せる」 ← 嘘になる。閉じたクライアントは復活できない
- ○ 「間違えて飛ばしても、潰しても、Ctrl+Z で戻せる」
- ○ 「エディタの Ctrl+Z が、画面のレイアウトに効く」

## 1. 競合調査(2026-10 時点)

| WM | 組み込み | 実態 |
|---|---|---|
| i3 / sway | **あり** (`i3-save-tree` + `append_layout`) | 公式docが「出力は手で編集しないと使えない」と明記。出力が valid JSON ですらない(コメント入り) |
| bspwm | あり (`bspc wm -d` / `-l`) | デバッグ用ステートダンプ。セッション跨ぎは想定外 |
| Hyprland | **なし** | サードパーティが9個以上乱立(winsnap / hyprstate / hyprflow / hypr-layout / hyprwake / hyprresurrect / HyprRestore …) |
| niri | **なし** | niri-session-manager / nirinit 等。niri 本体はワークスペース→出力の対応だけ記憶 |
| Windows | **なし** | Win11 Snap Layouts はグループ記憶で、1手戻す undo ではない |

### 読み取れること

- **需要は証明済み**。人が自分でスクリプトを書くほど欲しい
- **誰も解けてない**。解けてたら Hyprland 向けに9個は要らない
- **undo はどの OS にも無い**。痛みが OS 非依存なので、説明も OS 非依存にできる

### なぜ外部ツールは良くならないのか(ここが felis の勝ち筋)

既存ツールは全部コンポジタの**外**にいる。

1. `hyprctl clients -j` 等を叩いて状態をスクレイプ
2. 返るのは**平坦化された矩形リスト**。木の構造は失われてる
3. 復元時はアプリ起動とレースしながら move/resize を投げて形を再現

hyprstate や niri-session-manager が「**idempotent**」を売り文句にしてるのが証拠。
自然には冪等にならないから謳ってる。矩形から木を逆算してるので原理的にロスがある。

felis で木がそのまま値なら、save/restore は serde の往復。逆算なし、レースなし、
冪等性は構造から来る。

### 正直な上限

9個のツールが存在する = **外部ツールで実用上は足りてしまう**。
hyprresurrect で満足してる人はこの機能のために WM を乗り換えない。

→ これは**新規獲得の武器ではなく、「触った人が戻れなくなる」側の武器**。
獲得用の一行は別途必要(見た目か、まだ誰もやってないレイアウト挙動)。

## 2. 設計

### 2.1 表現を2つに分ける(最重要)

実行時表現(arena + parent ポインタ)を**そのまま serialize してはいけない**。
arena インデックスと `free_list` は確保順に依存する内部事情で、保存形式にすると
round-trip が壊れる。**i3 が手編集を要求するのは内部構造をそのままダンプしてるから**。

| 表現 | 用途 | 最適化対象 |
|---|---|---|
| `BspLayout`(arena, parent ポインタ) | 実行時 | 変更と上方向の探索 |
| `LayoutTree<T>`(再帰木、arena ID 無し) | 永続化 / スナップショット | 安定性と可読性 |

```rust
// layout/tree.rs (新規)
#[derive(Clone, Serialize, Deserialize)]
pub enum LayoutTree<T> {
    Leaf(T),
    Split {
        axis: Axis,
        ratio: f32,
        first: Box<LayoutTree<T>>,
        second: Box<LayoutTree<T>>,
    },
}

pub type Snapshot    = LayoutTree<WindowId>;        // undo用: 実体を指す
pub type SavedLayout = LayoutTree<Option<Binding>>; // ディスク用: マッチャ
```

`map_leaves` を1個付ければ `Snapshot → SavedLayout` は `id → binding` を渡すだけ。
**undo と保存が同じ型の別インスタンス**になる。

変換は `BspLayout ⇄ LayoutTree<WindowId>` の2方向(`snapshot()` / `restore()`)。

### 2.2 WindowId

```rust
// window.rs (新規)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct WindowId(u64);

pub struct WindowRegistry {
    next: u64,
    by_id: HashMap<WindowId, Window>,
    by_surface: HashMap<WlSurface, WindowId>,  // remove/grab から引くのに必要
}
```

これが全部の土台。木から smithay のライブハンドルを追い出す。

得られるもの:

1. `bsp.rs` が smithay 依存を失う(`Rectangle`/`Logical` は純粋な数学型)→ **Wayland 抜きでテストできる**
2. `find_leaf` の O(n) 全スキャンを `HashMap<WindowId, NodeId>` に置き換えられる
3. 木が値になる → 以降が全部乗る

### 2.3 Binding(保存した木への流し込み)

```rust
#[derive(Clone, Serialize, Deserialize)]
pub enum Binding {
    Any,                                   // 形だけ保存(機能A)
    AppId(String),                         // 中身込み(機能B)
    AppIdTitle { app_id: String, title: String },
}
```

`Any` だけ使えば「レイアウトテンプレート」、`Some` を入れれば「セッション」。
**データ構造は1個、流し込みポリシーが違うだけ。** A を先に作って B は後から足せる。

保存先: `~/.local/share/felis/layouts/<name>.json`

### 2.4 floating も分数で持つ

BSP の木は `ratio: f32` しか持たないので**解像度非依存**。
floating を絶対座標で保存すると別モニタで画面外に飛ぶ。なので揃える:

```rust
/// 出力領域に対する正規化矩形 (0.0..1.0)
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Rect01 { x: f32, y: f32, w: f32, h: f32 }
```

**現状の問題:** `FloatingLayout` は `Vec<Window>` だけで**座標を持ってない**
(`layout/floating.rs:58` の `layout()` は空 Vec を返す)。
真実は `Space` が持ってる(`grabs/move_grab.rs:34` が `space.map_element` で直接書いてる)。

必要な変更は **Phase 1 と同じ原則 = 真実を smithay から自分の値型に移す**:

```rust
pub struct FloatingLayout {
    windows: Vec<(WindowId, Rect01)>,
    focused: Option<usize>,
}
```

データの流れが逆転する:

- 今: `Space` が位置を持つ → `arrange()` が読んで書き戻す
- 後: `FloatingLayout` が位置を持つ → `arrange()` が `Space` に**書く**(`Space` は表示用ビュー)

`move_grab.rs` / `resize_grab.rs` はドラッグ結果を `FloatingLayout` に書き戻す。

### 2.5 Snapshot の最終形

```rust
#[derive(Clone, PartialEq)]  // PartialEq: before == after なら履歴に積まない
pub struct WorkspaceSnapshot {
    tiling: Option<LayoutTree<WindowId>>,
    floating: Vec<(WindowId, Rect01)>,
    focused: Option<WindowId>,  // undo で focus を操作箇所に戻す
}
```

undo も save も両方これ1個でカバー。save 用は葉を `Binding` に写すだけ。

### 2.6 History — 保存は1本、意味は per-workspace

```rust
pub struct History {
    entries: VecDeque<Entry>,   // ← 全体で1本(Felis 側に置く)
    future: Vec<Entry>,         // redo
    last: Option<(CmdKind, WorkspaceId, Instant)>,  // coalescing 判定用
}

struct Entry {
    /// この操作が触ったワークスペース。通常1個、ワークスペース間移動なら2個
    touched: SmallVec<[WorkspaceId; 2]>,
    before: Vec<(WorkspaceId, WorkspaceSnapshot)>,
}
```

`Ctrl+Z` = `entries` を後ろから見て、`touched` に**現在の**ワークスペースを含む
最初のエントリを戻す。

これで同時に成立する:

- デフォルトが per-workspace セマンティクス(予測可能)
- `undo-global` は「フィルタをかけず最後のエントリを取る」だけ。実質タダ
- **ワークスペース間のウィンドウ移動が自然に正しくなる** ← 履歴を各 `Workspace` に
  分けて持つと、片方だけ巻き戻る不整合が起きる

## 3. 決定事項

| 項目 | 決定 | 理由 |
|---|---|---|
| undo のスコープ | **per-workspace** がデフォルト。設定で切り替えない | 見えない undo は「壊れてる」と感じる。勝手なWS移動も同じく悪い。エディタがバッファ単位なのと同じ理由。設定項目は予測可能性を壊すだけ |
| `undo-global` | **別キーバインド**として出す(モードではなく) | 明示的に頼んだ移動なら期待通りになる。コアに分岐が入らない |
| undo の実装方式 | **スナップショット**。逆コマンドは採らない | 木は十数ノードしかない。逆コマンドは `Remove` の逆に位置情報が必要でバグの巣 |
| `History` の置き場所 | **`Felis`**(`Workspace` ではない) | エントリが複数WSにまたがるので `Workspace` 内では表現できない |
| `resize` の delta | **`f32` の比率**(0.02 等)。`i32` ピクセルから変更 | `ratio` は分数。ピクセルだとその Split の矩形幅が必要だが `geometry()` は矩形を保持してない(`layout/bsp.rs:66`)。ポインタドラッグ側は呼び出し側で `delta_px / area_width` を計算して渡す |
| floating の座標 | **`Rect01`(正規化)** | tiling と性質を揃えて解像度非依存にする |
| 保存形式 | JSON(valid JSON を保つ。i3 の逆) | 手編集を要求しないのが差別化点 |
| `crates/` ディレクトリ | **残す** | Phase 1 後に `layout/` が smithay 非依存になるので `crates/felis-layout` として独立させられる。OS時代の理由では不要になったが別の理由で正しくなる |
| コミット | **ユーザーがやる**。Claude は提案まで | コミット境界がユーザーのレビュー機構そのもの |

## 4. 未決事項(ユーザーが決める)

2026-10-30: 1〜5 は決定済み。内容は `docs/design.md` を参照。

1. ~~特異性の一文~~ → 決定済み
2. ~~`Binding` のマッチ規則~~ → 決定済み(app_id のみ、同一 app_id は深さ優先順)
3. ~~余り処理のポリシー~~ → 決定済み(葉は潰す、窓は最後の葉に add)
4. ~~coalescing の閾値~~ → 決定済み(focus は積まず snapshot に含める、before == after は積まない)
5. ~~`undo-global` を出すか~~ → 当面出さない(per-workspace のみ)
6. **floating を save の対象に含めるか**(undo は含めると決定。save は絶対座標問題があるので
   `Rect01` 化が前提)

## 5. フェーズ計画と分担

分担の原則は「難易度」ではなく **「間違いを誰が検出できるか」**:

- Claude = コンパイラ/テストが即座に誤りを捕まえる部分
- ユーザー = 「正しい」が『使って気持ちいいか』でしか判定できない部分

| # | 内容 | 担当 | 規模 | 触るファイル |
|---|---|---|---|---|
| 1 | `Window` → `WindowId` 化 + テスト基盤 | **Claude** | ~150行 | `window.rs`(新)、`layout/*`、`workspace.rs`、`state.rs`、`handlers/xdg_shell.rs`、`grabs/*` |
| 2 | `climb()` 抽出 + `resize()` 実装 | **ユーザー** | ~60行 | `layout/bsp.rs` |
| 3 | `FloatingLayout` が座標を持つ + grabs の書き戻し + `Rect01` | **Claude** | ~100行 | `layout/floating.rs`、`grabs/*`、`state.rs` |
| 4 | `LayoutTree` + serde + IO(`fill` を穴で残す) | **Claude** | ~200行 | `layout/tree.rs`(新)、`layout/bsp.rs`、`state.rs` |
| 5 | `fill`(流し込みアルゴリズム) | **ユーザー** | ~80行 | `layout/tree.rs` |
| 6 | `History` + undo + coalescing + user/system の線引き | **ユーザー** | ~80行 | `history.rs`(新)、`state.rs`、`input.rs` |

### 効くパターン: 穴を空けた足場

Claude が型と IO を書いて、**アルゴリズム箇所を仕様コメント付きの `todo!()` で残す**。
テストは先に書いて**全部赤**にしておく。ユーザーが埋めると緑になる = 仕様が実行可能な形で手元にある。

残した `todo!()` は「ユーザー担当」の印。Claude は勝手に埋めない。

### 運用ルール

- 同じファイルを同時に触らない。「今これ書いてる」と言われたファイルは Claude は触らない
- Claude は作業後ビルドが通る = コミット可能な状態で置き、差分の要点を伝える
- `NOTES.md` の流儀(ユーザーが設計を文章で書く → Claude が実装)を維持

## 6. 実装時の落とし穴

### 6.1 `app_id` は `new_toplevel` 時点では未設定

`handlers/xdg_shell.rs:37` の時点ではクライアントがまだ `app_id` を送ってない。
**保存時に遅延して読む**:

```rust
with_states(surface.wl_surface(), |states| {
    states.data_map.get::<XdgToplevelSurfaceData>()
        .unwrap().lock().unwrap().app_id.clone()
})
```

`xdg_shell.rs` は既に `with_states` と `XdgToplevelSurfaceData` を import 済み。

### 6.2 流し込みの順序

**具体的な `Binding` を持つ葉を先に確定させる。その後 `Binding::Any` に残りを詰める。**
逆にすると `Any` が貪欲に食って具体指定が入る窓を失う。ここだけバグりやすい。

手順:

1. 生存ウィンドウを `(WindowId, app_id, title)` で列挙
2. 葉を深さ優先(`first` → `second` = 左上から右下)で辿る
3. 具体 `Binding` の葉を先にマッチ
4. `Any` の葉に残りを順に詰める
5. 余った葉 → 潰す(`remove` の縮約処理を再利用。NOTES.md §6)
6. 余ったウィンドウ → 最後の葉に `add`(昇格処理を再利用。NOTES.md §1)

5と6が既存処理の再利用なので新しいアルゴリズムは不要。

### 6.3 coalescing は「積まない」が正しい

連続した同種操作を1ステップにまとめる際、**直前のスナップショットを差し替えるのではなく、
新しく積まない**。連続の起点に戻りたいので。

```rust
if self.last.map_or(false, |(k, ws, t)| k == kind && ws == cur_ws
                                      && t.elapsed() < Duration::from_millis(500)) {
    // 積まない
} else {
    self.entries.push_back(entry);
}
```

### 6.4 ユーザー起因だけ記録する

`input.rs` 経由だけ積む。`new_toplevel` 由来の `add` は**木は変えるが履歴に積まない**。
これを分けないと、裏で通知ポップアップが開くたびに履歴がゴミで埋まって
Ctrl+Z が無関係な所に飛ぶ。

```rust
impl Felis {
    /// input.rs から。履歴に積む
    pub fn apply_user(&mut self, cmd: LayoutCmd) { ... }
    /// handlers から。積まない
    pub fn apply_system(&mut self, cmd: LayoutCmd) { ... }
}
```

### 6.5 `climb` は3箇所で共通

`layout/bsp.rs:175` の `neighbor_leaf` がやってる「葉から登って dir 方向に曲がれる
最初の親を探す」は `focus` / `move_window` / `resize` で同一ロジック。先に抽出する:

```rust
/// from から登り、dir 方向に曲がれる最初の Split を返す
fn climb(&self, from: NodeId, dir: Direction) -> Option<(NodeId, bool /* from が first 側か */)>
```

`from_first` が効くのは resize で、「first 側にいて右に伸ばす = ratio 増」
「second 側にいて右に伸ばす = この Split では伸ばせないので更に登る」の判定に使う。

### 6.6 閉じたアプリは復活できない

クライアントプロセスが終了し `wl_surface` が破棄された時点で、コンポジタは中身を
一度も所有してないので原理的に無理。undo できるのは**自分が持ってる状態 = 木の形だけ**。

| やったこと | undo | 理由 |
|---|---|---|
| WS 移動 / resize / swap | ○ 完全に戻る | 木の変更だけ。クライアントは生きてる |
| アプリを閉じた | △ 枠だけ | 木の穴は埋め直せるが中身は無い |

機能B(`app_id` 束縛)で「再起動して同じスロットに配置」まではできる。
ブラウザは自前のセッション復元を持つので体験が良くなるが、ビルドが走ってた
ターミナルは何も戻らない。**宣伝するなら「レイアウトの undo」であって
「ウィンドウの undo」ではない。**

参考: Wayland 側に `xdg-session-management` の提案があるが、まだ提案段階で、
確定しても解決するのは配置の復元だけ(アプリ状態は戻らない)。

## 7. テスト戦略

Phase 1 後、Wayland 抜きで書ける:

```rust
#[test]
fn three_windows_tile_without_gaps() {
    let mut l = BspLayout::new();
    l.add(WindowId(1)); l.add(WindowId(2)); l.add(WindowId(3));
    let rects = l.layout(Rectangle::new((0,0).into(), (1920,1080).into()));
    assert_no_overlap(&rects);
    assert_covers_exactly(&rects, (1920, 1080));
}
```

property test の不変条件(コマンド列をランダム生成して検査):

- 全矩形が重ならない
- 全矩形の面積合計 = 画面面積
- 内部ノードは必ず子2個、葉以外に `WindowId` が無い
- `free_list` の ID がどこからも参照されてない

これが回ってると `resize` や流し込みのエッジケースを実機で目視せずに潰せる。
fractional scaling の正しさも `area` を変えて同じ検査を流すだけになる。

## 8. リポジトリ整理(OS 時代の残骸)

元々 OS としてやる予定だったものを WM だけにスコープ縮小したため、構成に残骸がある。

| 場所 | 状態 | 対応 |
|---|---|---|
| `docs/design.md` | OS/DE 時代の内容(バー=Zig、ランチャー自作、flatpak、「NixOS上のDE」) | **ユーザーが書き直す**。プロジェクトのスコープと特異性の一枚に |
| `crates/felis-compositor/docs/design-compositor.md` | **内容が現実と逆**。「フローティング型」「fvwm の wayland 版」と書いてあるがコードは BSP タイリング優先(`workspace.rs:34` は tiling しか返さない) | 上に統合して削除 |
| `Cargo.toml` の `[workspace.dependencies]` | `iced` / `zbus` / `tokio` が未使用(OS時代の遺物) | 削除。**`serde` と `dirs` は残す**(Phase 4 で使う) |
| `crates/` が1メンバー | 潰したくなるが | **残す**(§3 参照) |

ドキュメントは3箇所 → 2箇所に:

- `docs/design.md` — プロジェクト全体のスコープと特異性(ユーザーが書く)
- `src/<module>/NOTES.md` — モジュールごとの設計メモ(今の流儀のまま、これが一番機能してる)

### ⚠️ 再編の順番

ディレクトリ再編は**別マシンに未マージの作業がある状態でやると最悪**。
リネーム検出はヒューリスティックなので、片方が編集・片方が移動すると
`deleted by us / modified by them` になり、相手の編集を新パスに手で再適用する羽目になる。
両方が別々に再編したら実質解決不能。

Phase 1(8ファイル触る)も同じ性質。手順:

1. デスクトップPC側を push / マージして**収束させる**(`git fetch && git log --oneline main..origin/main` で確認)
2. 再編を**1コミット**で
3. デスクトップ側で先に pull してから作業再開

## 9. 設計の原則(ShojiWM から持ってくるもの)

ShojiWM の「TS で見た目を書けるのに速度が落ちない」理由は、TS が速いからではなく
**TS がフレームループの中にいないから**。TSX は合成ツリーの**宣言**を一回作るだけで、
毎フレーム走るのは Rust + GPU。awesome が Lua でも軽いのと同じ構造。

> **スクリプト層は「宣言」する。実行は Rust が毎フレームやる。**

これが崩れる(スクリプトを per-frame コールバックにする)と、言語が何であれ終わる。
felis で何をやるにせよこの原則は持っていく。

## 10. 後回しにするもの

| 項目 | 理由 |
|---|---|
| プラグイン ABI(`.so`) | Rust に安定 ABI が無い。更新ごとに壊れ、プラグインのクラッシュが felis のクラッシュとして見える。API を公開した瞬間に内部リファクタが凍る。**需要が実際に出てから** |
| 埋め込み VM(mlua / rhai / WASM) | 上と同じ理由で早すぎる。やるなら river モデル(レイアウト生成を別プロセスに出す)が Rust 的に一番安い |
| 見た目(シェーダ、角丸、blur) | smithay 0.7 に API はある(`GlesRenderer::compile_custom_texture_shader` / `compile_custom_pixel_shader` + `PixelShaderElement`)。軸が立ってから装飾として足す |
| IPC ソケット / layer-shell | 実装は安く効果は大きい(waybar が動く)。ただし特異性にはならない |
| blur | オフスクリーン FBO + 描画順の再設計が必要でアーキテクチャ変更。一番最後 |

### 参考: smithay 0.7 のシェーダ API(確認済み)

- `GlesRenderer::compile_custom_texture_shader(src, uniforms)` → `GlesTexProgram`
  — ウィンドウのテクスチャに後処理(角丸・ボーダー・グロー)
- `GlesRenderer::compile_custom_pixel_shader(src, uniforms)` → `GlesPixelProgram`
  + `PixelShaderElement::new(shader, area, opaque_regions, alpha, uniforms, kind)`
  — 任意の矩形をシェーダだけで塗る(背景・インジケータ)

どちらも `v_coords` / `size` / `alpha` / `tint` が渡る。`#version` は書かない(100 扱い)。
テクスチャシェーダ側は `//_DEFINES` の行が必要。差し込み先は `winit.rs` の
`backend.bind()` 以降の描画ループ。

## 出典

- [i3: Layout saving in i3](https://i3wm.org/docs/layout-saving.html)
- [i3-save-tree(1)](https://man.archlinux.org/man/extra/i3-wm/i3-save-tree.1.en)
- [winsnap](https://github.com/tqdesign/winsnap) / [hyprstate](https://github.com/flaviomilan/hyprstate) / [hyprresurrect](https://github.com/FillipdotS/hyprresurrect)
- [niri-session-manager](https://github.com/LarsArtmann/niri-session-manager) / [awesome-niri](https://github.com/niri-wm/awesome-niri)
- [i3-layout-manager](https://github.com/klaxalk/i3-layout-manager)
