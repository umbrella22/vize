# Vize Examples

Vizeのツールをローカル環境で試すためのサンプル集です。

## 前提条件

プロジェクトルートで以下を実行してビルドしておく必要があります：

```bash
vp env install
vp install
vp run --workspace-root cli  # vize CLIコマンドを有効化
```

または Cargo から直接実行：

```bash
cargo build --release
```

---

## CLI Examples

`examples/cli/` ディレクトリにはCLIツールを試すためのサンプルVueファイルが含まれています。

### ファイル構成

| ファイル              | 説明                                |
| --------------------- | ----------------------------------- |
| `src/App.vue`         | 正常にフォーマット済みのVueファイル |
| `src/Unformatted.vue` | フォーマットが必要なVueファイル     |
| `src/HasErrors.vue`   | リントエラーを含むVueファイル       |

### フォーマッター (vize fmt)

```bash
# フォーマットが必要かどうかをチェック
vize fmt examples/cli/src/*.vue --check

# フォーマット結果を表示（ファイルは変更しない）
vize fmt examples/cli/src/Unformatted.vue

# ファイルに書き込み
vize fmt examples/cli/src/Unformatted.vue --write

# オプション付き
vize fmt examples/cli/src/*.vue --single-quote --no-semi --print-width 80
```

**オプション一覧：**

| オプション       | 説明                         | デフォルト |
| ---------------- | ---------------------------- | ---------- |
| `--check`        | 変更が必要な場合にエラー終了 | -          |
| `--write`, `-w`  | ファイルに書き込み           | -          |
| `--single-quote` | シングルクォートを使用       | false      |
| `--no-semi`      | セミコロンを省略             | false      |
| `--print-width`  | 行の長さ                     | 100        |
| `--tab-width`    | インデント幅                 | 2          |
| `--use-tabs`     | タブを使用                   | false      |

### リンター (vize lint)

```bash
# リントエラーを表示
vize lint examples/cli/src/*.vue

# JSON形式で出力
vize lint examples/cli/src/HasErrors.vue --format json

# 警告の上限を設定
vize lint examples/cli/src/*.vue --max-warnings 5

# サマリーのみ表示
vize lint examples/cli/src/*.vue --quiet
```

**オプション一覧：**

| オプション       | 説明                 | デフォルト |
| ---------------- | -------------------- | ---------- |
| `--format`, `-f` | 出力形式 (text/json) | text       |
| `--max-warnings` | 警告数の上限         | -          |
| `--quiet`, `-q`  | サマリーのみ表示     | false      |
| `--fix`          | 自動修正（未実装）   | false      |

### LSPサーバー (vize lsp)

```bash
# stdio で起動（エディタ連携用）
vize lsp

# TCPポート指定
vize lsp --port 3000

# デバッグログ有効
vize lsp --debug
```

**エディタ設定例 (VS Code):**

`.vscode/settings.json`:

```json
{
  "vize.lsp.path": "/path/to/vize",
  "vize.lsp.args": ["lsp", "--debug"]
}
```

---

## Vite + Musea Example

`examples/vite-musea/` ディレクトリにはVite + Museaを使ったコンポーネントギャラリーのサンプルが含まれています。

### セットアップ

```bash
cd examples/vite-musea
pnpm install
pnpm dev
```

### 使い方

1. `pnpm dev` で開発サーバーを起動
2. ブラウザで `http://localhost:5173` を開く
3. コンポーネントギャラリーは `http://localhost:5173/__musea__` で確認可能

### ファイル構成

| ファイル                        | 説明                                    |
| ------------------------------- | --------------------------------------- |
| `src/components/Button.vue`     | Buttonコンポーネント                    |
| `src/components/Button.art.vue` | Museaのアートファイル（バリアント定義） |
| `vite.config.ts`                | Vite + Musea設定                        |

### Art ファイルの書き方

`.art.vue` ファイルはコンポーネントのバリアントを定義します：

```vue
<art title="Button" component="./Button.vue" category="Components" status="ready">
  <variant name="Default" default>
    <Button>Default Button</Button>
  </variant>
  <variant name="Primary">
    <Button variant="primary">Primary Button</Button>
  </variant>
</art>

<script setup lang="ts">
import Button from "./Button.vue";
</script>
```

**`<art>` 属性：**

| 属性        | 説明                                |
| ----------- | ----------------------------------- |
| `title`     | コンポーネントのタイトル（必須）    |
| `component` | 対象コンポーネントへのパス          |
| `category`  | カテゴリ                            |
| `status`    | ステータス (draft/ready/deprecated) |

**`<variant>` 属性：**

| 属性       | 説明                                    |
| ---------- | --------------------------------------- |
| `name`     | バリアント名（必須）                    |
| `default`  | デフォルトバリアントとしてマーク        |
| `skip-vrt` | VRT（Visual Regression Test）をスキップ |

---

## トラブルシューティング

### `vize` コマンドが見つからない

```bash
# vp run から CLI を有効化
vp run --workspace-root cli

# または直接 cargo run を使用
cargo run --release -- fmt examples/cli/src/*.vue
```

### ネイティブバインディングのエラー

Museaプラグインを使用する場合、`@vizejs/native` がビルドされている必要があります：

```bash
vp run --workspace-root build:native
```
