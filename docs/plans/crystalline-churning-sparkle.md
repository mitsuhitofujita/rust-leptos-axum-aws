# 新規行動種類追加ページ — 実装計画

> `docs/README.md` のドキュメントモデルに `docs/plans/` は含まれません（Work Log /
> Decision Record / Design Document の3種のみ）。このファイルはハーネスが指定した
> 一時的な作業計画であり、正典は `docs/work/2026-08-10-add-action-type-page.md`
> です。実装中の記録はそちらに追記し、このファイルは作業完了時に破棄します。

## Context

`docs/design/page-layouts.md` の "Add action type" は設計として確定しているのに、
実装が存在しません。ルート `/action-types/new` は未定義で、`crates/app/src/app.rs`
の `NotFound` が「まだ作られていない画面」として応答している状態です。
`crates/app/src/dashboard.rs:46` のアカウントコントロールは既に `/action-types`
を指しており、そこも同じフォールバックに落ちます。

サーバ側はさらに手前で止まっています。`docs/design/persistence.md` が記す通り
テーブルは作成済みで空、`crates/server` は AWS SDK 依存を持たず、Cognito の
`sub` を取り出す経路も存在しません。

この作業のゴールは、空のフォームから DynamoDB に1件の行動種類が保存されるまでの
縦一本を通すことです。範囲・前提・スコープ外は Work Log の Interpretation に
記載済みで、そこで確定した判断は次の3つです。

- 縦に全部通す（画面 → API → DynamoDB、Cognito `sub` の導出を含む）
- 遷移先として最小の `/action-types` 一覧も作る
- アイコンは `lucide-leptos` の少数カテゴリに限定する

追加で、この計画作成中に確定した2点:

- ローカル開発は `TABLE_NAME` 未設定時にインメモリストアを使う（AWS 資格情報不要）
- リクエストコンテキストヘッダが無い場合は固定の開発用 `sub` を使う

### 調査で分かった前提

- **インフラ変更は不要。** `infra/api/apigateway.tf` は `POST /api/{proxy+}` を
  authorizer 配下に既にルートしており、`POST` は `local.api_methods` 経由で CORS
  にも入っています。`infra/api/iam.tf` は `PutItem` と `Query` を許可済み、
  `infra/api/lambda.tf` は `TABLE_NAME` を既に渡しています。
- **`sub` は Lambda Web Adapter が転送するヘッダから取る。** LWA は API Gateway の
  リクエストコンテキストを `x-amzn-request-context` ヘッダに JSON で入れます
  （`lambda_http::RequestContext` は `#[serde(untagged)]` なので、HTTP API v2 の
  コンテキストがそのままトップレベルの JSON になります）。読む場所は
  `authorizer.jwt.claims.sub`。サービスはトークンの署名を検証しません — 検証済み
  なのは API Gateway の JWT authorizer です（DR-0010）。
- **Lucide のカテゴリはこのアプリの用途と形が合っていない。** デザイン参照の8個
  だけで8カテゴリにまたがります（詳細は Work Log の Progress）。
- **`lucide-leptos` 3.26.0 は leptos 0.8.0 に依存**しており、ワークスペースの
  0.8.20（DR-0002）と両立します。アイコンは生成済みコンポーネントで、名前から
  引く実行時ルックアップは持たないため、対応表はこちらで生成します（DR-0014）。

---

## 1. アイコンカタログ

**依存の追加。** ルート `Cargo.toml` に `lucide-leptos = "3.26.0"`。
`crates/app/Cargo.toml` で有効化するカテゴリは、デザイン参照の8個を覆う8つに、
行動記録で使いそうなものを足した14個から始めます:

```
people weather text time transportation sports buildings navigation
nature animals food-beverage medical home travel
```

**ジェネレータ `crates/icongen`**（ワークスペースメンバー、ネイティブ bin）。

- `crates/app/Cargo.toml` を読んで `lucide-leptos` の `features` 配列を取り出す。
  カテゴリ一覧の正本は Cargo.toml 一箇所だけにする（`toml` クレートを使う）。
- cargo レジストリのソースキャッシュから `lucide-leptos-3.26.0/src/lib.rs` を読み、
  複数行にまたがる `#[cfg(...)] mod <name>;` を解析して、有効カテゴリを1つでも
  含むモジュールだけを残す。
- スネークケースのモジュール名から、canonical な kebab-case 名（`person_standing`
  → `person-standing`）と表示名（→ `Person Standing`）を導く。
- 2つのファイルを出力する:
  - `crates/shared/src/icon_names.rs` — `pub const ICON_NAMES: &[&str]`（ソート
    済み）と `pub fn is_known(name: &str) -> bool`（二分探索）。ビューを持たない
    ので wasm とネイティブの両方でコンパイルでき、サーバ側の検証がこれを使う。
  - `crates/app/src/icon_catalog.rs` — `pub struct Icon { name, display, view: fn() -> AnyView }`
    の `pub static CATALOG: &[Icon]`（名前順）と `pub fn find(name) -> Option<&'static Icon>`。
    400個規模の巨大な `match` を避けるため、関数ポインタの配列にする。
- `just icons` で手動再生成。ピン留めを動かしたときに走らせる（DR-0014）。

**既存コードの追従。**

- `crates/app/src/icons.rs` の `ActivityGlyph` は手描き10グリフをやめて
  `icon_catalog::find` に委譲。未知の名前は今の汎用グリフにフォールバック。
- `crates/server/src/main.rs` のダミーレコードのアイコン id（`running`、`water`
  …）は Lucide 名ではないため、全部フォールバックに落ちます。カタログ内の名前に
  差し替える（`footprints`、`droplets`、`book-open`、`timer`、`bike`、`dumbbell`、
  `graduation-cap` など、生成結果で存在を確認して決める）。
- `THIRD_PARTY_NOTICES.md` に `lucide-leptos` 自身の MIT 表示を追加する。同ファイル
  末尾の段落が既にそれを求めています。

**計測。** 追加前後で `trunk build --release` の wasm サイズを測り、Work Log に
記録する。features はモジュールがコンパイルされるかを決めるだけで、生成した
`CATALOG` が参照しないアイコンは release ビルドで落ちる見込みがあります。結果
次第でカテゴリ列挙のままいくか、DR-0014 が挙げていた「必要な分だけ生成する」
フォールバックに寄せるかを決めます。

## 2. アイコンピッカー

新規 `crates/app/src/icon_picker.rs`。`IconField` コンポーネント1つに、コンパクト
セレクタとモーダルの両方を収める（編集画面が後からそのまま再利用できる形）。

- props は `icon: RwSignal<String>`。フォームが値を所有し、ピッカーは適用時にだけ
  書き込む。
- `<dialog>` は `NodeRef<leptos::html::Dialog>` + `web_sys::HtmlDialogElement` の
  `show_modal()` / `close()`。フォーカスの閉じ込めと Escape はネイティブ任せ
  （DR-0013）。
- 行は `CATALOG` から一度だけ描画し、各行の `hidden` を検索クエリ由来の派生
  シグナルに束ねる。参照実装の JS と同じ挙動で、キー入力ごとにリストを作り直さない。
- 選択は `staged: RwSignal<String>` に貯め、`Use selected icon` でだけ `icon` へ
  書き込んで閉じる。閉じる／Escape は `icon` を触らない。
- `on:close` で `aria-expanded="false"` に戻し、トリガーへフォーカスを返す。
- 件数は真の一致数を出す Memo、0件は明示的な空状態。
- `crates/app/Cargo.toml` の `web-sys` features に `HtmlDialogElement`、
  `HtmlInputElement`、`HtmlElement` を追加。
- `style/main.css` に `docs/design/html/action-types-create.html` の
  `.icon-select` / `.selected-icon` / `.sr-only` / `.icon-dialog*` / `.search-field`
  / `.icon-search` / `.results-meta` / `.icon-result*` / `.result-*` /
  `.empty-results` / `.apply-icon-button` を移植。

## 3. 作成画面

新規 `crates/app/src/action_types.rs`。

- `crates/app/src/app.rs` に `/action-types/new` を追加し、`RequireAuth` で包む
  （DR-0011）。
- 構成は `docs/design/html/action-types-create.html` の通り: `page-heading`、
  1枚の `form-card` に3フィールド、`form-actions` に solid accent の
  `Create action type` と `Cancel`。
- 各フィールドは `RwSignal<String>`。クライアント側の必須チェックはサーバが行う
  検証の写しであって、検証そのものではない。
- 送信中はボタンを無効化し、失敗は既存の `.error-message` で画面に出す
  （`dashboard.rs` と同じで、握り潰さない）。
- `.create-form` / `.form-card` / `.field` / `.field-label` / `.text-input` /
  `.field-help` / `.form-actions` / `.primary-button` / `.cancel-link` を移植。

この時点でフォームは完成しますが送信先がありません。4と5がそれを与えます。

## 4. サーバが書けるようになる

**`crates/shared`** — `NewActionType { name, unit, icon }` を追加。
`icon_names.rs` を `lib.rs` から公開。

**`crates/server`** — 依存追加: `aws-config` 1.10、`aws-sdk-dynamodb` 1.120、
`ulid` 3.0、`time`（`created_at` の固定幅 RFC 3339 生成）、`serde_json`。
1ファイルのままにせず分割する:

| ファイル | 役割 |
| --- | --- |
| `main.rs` | ルータ、`AppState`、起動時のストア選択 |
| `identity.rs` | `x-amzn-request-context` → `sub` |
| `store.rs` | ストア。`enum Store { Dynamo(..), Memory(..) }` |
| `action_types.rs` | ハンドラと検証 |

- **ストアは trait object ではなく enum。** 実装は2つしかなく、`async fn` を
  dyn-safe にするための `async-trait` 依存を持たずに済みます。
  `TABLE_NAME` が設定されていれば `Dynamo`、未設定なら `Memory`（`Mutex` 越しの
  `HashMap`）。未設定値が「壊れたもの」ではなく「動くもの」を意味するという
  DR-0008 の方針と揃います。
- **書き込み**は `persistence.md` の通り: `pk = USER#<sub>`、`sk = TYPE#<ulid>`、
  属性 `name` / `unit` / `icon` / `created_at`。ULID はサービスが生成し、API には
  裸の ULID だけを `id` として出す。
- **識別子**は `x-amzn-request-context` の JSON から `authorizer.jwt.claims.sub`
  を取る。必要な部分だけの最小の struct に `serde_json` でデコードする。ヘッダが
  無ければ固定の開発用 `sub` を使う（ローカル専用の経路で、本番では API Gateway
  が常にヘッダを付ける）。**Decision Record を1本書く。**
- **検証**: `name` / `unit` は trim して空を拒否、長さ上限（`name` 64、`unit` 16
  を提案 — どの文書も定めていないのでここで決めて Work Log に記す）、`icon` は
  `shared::icon_names::is_known` で照合。違反は 400。
- `crates/app/src/api.rs` に `post_json` を `get_json` の隣に足す。401 の扱いは
  そのまま継承する。

## 5. 一覧と2つの出口

- `GET /api/action-types` → `Vec<ActionType>`。`pk = USER#<sub>` と
  `begins_with(sk, "TYPE#")` の Query。
- `/action-types` を `RequireAuth` 配下に追加。構成は
  `docs/design/html/action-types-list.html` の populated state
  （`.add-button` / `.type-list` / `.type-link` / `.type-icon` / `.unit-value` /
  `.edit-icon` / `.type-count`）。行のリンク先は編集画面（未実装なので当面は
  `NotFound` が答える）。
- Page Layouts が定義していない空状態を1つ書く。新規アカウントは必ずここから
  始まるため必要で、退役時に Page Layouts へ提案する。
- `Cancel` と作成成功はどちらも `/action-types` へ遷移する。

## 6. 退役（Retirement）

Design Document の更新案を作り、確認を取ってから完了とする（`docs/README.md` の
Ownership に従い、上書きは人が確認する）。

- `frontend.md` — 新モジュール、ルート、依存（`lucide-leptos`、追加した web-sys
  features）、`post_json`
- `persistence.md` — 冒頭の「まだ誰もテーブルを読み書きしていない」という注記
- `workspace.md` — `crates/icongen` と `just icons`
- `page-layouts.md` — 一覧の空状態
- `index.md` — backend の Design Document を起こす時期かどうか
- Decision Record — (a) サービスが呼び出し元の識別子をどう得るか（ローカルの
  代替 `sub` を含む）、(b) 計測結果が DR-0014 の想定を変えるならアイコンカタログ
  のコストについて

---

## Verification

各タスクの終わりに:

```
just fmt-check && just lint && just check && just test
```

縦の確認（タスク5完了後）:

1. `just dev-api` と `just dev-web` を並べて起動する（どちらも設定不要。API は
   `TABLE_NAME` 未設定なのでインメモリ、SPA は `COGNITO_*` 未設定なのでサインイン
   無効 = `RequireAuth` は `Disabled` を通す）。
2. `/action-types` を開く → 空状態。`Add action type` → `/action-types/new`。
3. アイコンフィールドを開く → 検索にフォーカスが載る、絞り込みで件数が変わる、
   0件で空状態、Escape で値が変わらずトリガーにフォーカスが戻る、
   `Use selected icon` で値が変わる。
4. 名前・単位を入れて作成 → `/action-types` に戻り、行が増えている。
5. 空の名前、空白だけの単位、長すぎる名前を送って 400 が画面に出ることを確認する。
6. `curl -X POST localhost:3000/api/action-types` に不正な `icon` を渡して 400 に
   なることを確認する（ピッカーを通らない経路の検証）。
7. `trunk build --release` の wasm サイズをタスク1の前後で比較し、Work Log に記録。

実テーブルに対する確認（`aws sts get-caller-identity` が通る状態が必要 — 現在は
セッション期限切れ）は `just deploy-api` / `just deploy-web` の後に行う。ここは
ユーザの判断に委ねる。
