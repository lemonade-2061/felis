# BSPレイアウト設計メモ

Hyprlandの`dwindle`みたいなタイル型レイアウトをBSP(2分木)で作るための覚書。

## 木が表してるもの

各ノード = 画面領域(矩形)の分割。ウィンドウは必ず**葉**にしか居ない。

- **葉(Leaf)** = 実際のウィンドウ1個
- **内部ノード(Split)** = 「ある矩形を縦 or 横に、ある比率で2つに割る」ルールだけを持つ（ウィンドウは持たない）

木をたどると画面全体の矩形が再帰的に小さい矩形へ分割されていく。

```
        Split(Horizontal)        ← 縦線で左右に
        /            \
     Leaf A        Split(Vertical)   ← 横線で上下に
                    /        \
                 Leaf B    Leaf C

┌──────┬──────┐
│      │  B   │
│  A   ├──────┤
│      │  C   │
└──────┴──────┘
```

## 構造体

arena方式（`Vec`の添字をポインタ代わり）。`Rc<RefCell>`は地獄なので使わない。niri等もこの方式。

```rust
type NodeId = usize;

enum NodeKind {        // Kind = 「種類」。ノードが Leaf か Split か を区別するラベル
    Leaf  { window: Window },
    Split { axis: Axis, ratio: f32, left: NodeId, right: NodeId },
}

struct BspNode {
    kind: NodeKind,
    parent: Option<NodeId>,   // ★上に登るために必須。共通部分なのでkindの外に出してる
}

struct BspLayout {
    nodes: Vec<Option<BspNode>>,  // arena
    free_list: Vec<NodeId>,       // 削除済みスロット再利用
    root: Option<NodeId>,
    focused: Option<NodeId>,      // フォーカス中の「葉」
}
```

- `parent`が肝。focus/move/resizeは全部「葉から上に登る」操作。子→親リンクが無いと毎回ルートから探索になる。
- `Window`は内部ノードに持たせない（だからNodeKindで分けてる）。
- `BspNode{kind,parent}` と分けたのは「種類に関係なく全ノードがparentを持つ」から共通部分を外出ししただけ。共通部分が無いなら enum 一個でもいい。

### Axis

内部ノードが矩形を**どっち向きの線で割るか**を1個だけ持つ。

```rust
enum Axis {
    Horizontal,  // ウィンドウが横に並ぶ = 仕切り線は縦（｜）
    Vertical,    // ウィンドウが縦に並ぶ = 仕切り線は横（―）
}
```

- 「並ぶ方向で呼ぶ流儀」を採用（多くのWMがこれ）。線の向きで呼ぶ流儀とは逆になるので注意。`split`計算でバグらないようコメント必須。
- 1ノードは2分割しかしないので向きの値は1つで十分。多方向の分割は**木の階層**で表現する（十字4分割みたいなことはしない）。
- レイアウト構造の`Axis`と、ユーザー操作の向き`Direction`(Left/Right)は別物。混同しない。

## アルゴリズム

### 1. 追加 (add / insert) ← 木を変えるだけ
フォーカス中の葉を Split に「昇格」させ、下にぶら下げる。
```
追加前:  parent ── target(葉A)
追加後:  parent ── Split ── target(葉A)
                        └── new_leaf(葉B)
```
- target の親リンクを Split に付け替え、targetの親の left/right のうち target を指してた側を Split に差し替える。
- Splitの`axis`は「割る矩形が横長なら縦割り、縦長なら横割り」で決めると自然(Hyprlandデフォルト)。
- 最初の1枚だけは特別扱い: そのまま root にする。
- 普通は追加後に新ウィンドウへフォーカス移動。

### 2. ジオメトリ計算 (arrange / apply) ← 並べ直す
ルートに画面全体の矩形を渡し、再帰で下りながら各葉に矩形を確定。O(ノード数)。
```
fn layout(node, rect):
  match kind:
    Leaf{window} -> 葉に rect を割り当て → space.map_element / window.configure
    Split{axis, ratio, l, r} ->
      let (r1, r2) = rect.split(axis, ratio)
      layout(l, r1); layout(r, r2)
```
木が変わるたびに1回回す。

### 3. 方向フォーカス (focus(dir))
葉から親へ登り、`Split.axis`が動きたい方向と一致し、かつ自分が`left`側にいる最初の親を探す → その`right`部分木へ降りて端の葉を選ぶ。
（矩形の中心距離で空間的に最も近い葉を選ぶ実装もあり）

### 4. 移動 (move_window(dir))
部分木を切り離して別の場所に繋ぎ直す(detach→re-attach)。最初は隣とのswapで楽をするのがおすすめ。

### 5. リサイズ (resize(dir, delta))
該当する`Split.ratio`を増減するだけ。どのSplitを動かすかはfocusと同じく葉から登って方向の合う親を探す。`clamp(0.05, 0.95)`程度に制限。

### 6. 削除 (remove)
葉を消すと親Splitが子1個になるので、**親を残った兄弟で置き換えて潰す**(木を縮約)。忘れると無意味な内部ノードが溜まる。消したIDは`free_list`へ返す。

## 「追加したらどうするか」= 2段階

addだけ書いても画面は変わらない。位置は smithay の `Space` が持ってるので反映が要る。

```
new_toplevel
   ├─① layout.add(window)              木に葉を挿入（構造を変えるだけ）
   └─② layout.arrange(画面矩形, space)  木をたどって各葉の矩形を計算 → space に反映
```

## 実装順のおすすめ

1. `Felis` 構造体に `layout: BspLayout` フィールドを持たせる（今は `self.space` しか無い）
2. `WindowNav` トレイトに `fn add(&mut self, window: Window)` を追加
3. **add → arrange → remove** を動かして「開く / タイル表示 / 閉じる」を回す
4. focus / move / resize は木探索の応用なので後から足す

## 現状(2026-06-25)

- `layout/bsp.rs`: `BspLayout` の枠だけ。`nodes: Vec<Option<Node>>` (`Node=usize`) は中身を表現できないので上記の `NodeKind` 構造に直す必要あり。focus/move/resize は `todo!()`。
- `layout/floating.rs`: 配列ベースの単純な前後フォーカスのみ実装済み。
- `layout/scroll.rs`: 空。
- `handlers/xdg_shell.rs:37` `new_toplevel`: 今は `space.map_element((0,0))` するだけでレイアウト未接続。

## 参考実装

- Hyprland `DwindleLayout`（C++、生ポインタ）
- niri / leftwm のレイアウト周り（Rust、arena方式で読みやすい）
