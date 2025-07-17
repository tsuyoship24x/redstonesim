# simulate_py 入力JSON仕様

`simulate_py` 関数に渡す文字列は `SimRequest` 構造体を JSON として表現したものです。
以下に基本的なフォーマットと各フィールドの意味を示します。

## ルート構造
```json
{
  "ticks": 5,
  "early_exit": true,
  "world": {
    "blocks": [
      { "x": 0, "y": 0, "z": 0, "type": "lever", "on": true },
      { "x": 1, "y": 0, "z": 0, "type": "dust",  "power": 0 },
      { "x": 2, "y": 0, "z": 0, "type": "lamp",  "on": false }
    ]
  }
}
```

- **ticks**: シミュレーションを最大で何 tick 実行するかを指定します。
- **early_exit**: `true` の場合、状態変化が無くなり内部タイマーも停止した時点でシミュレーションを終了します。省略した場合は `true` になります。
- **world.blocks**: ブロック一覧を配列で指定します。各要素はブロックの座標と種類を表します。

## ブロック指定
各ブロックは以下のように座標 (`x`, `y`, `z`) と `type` を持ち、種類に応じた追加フィールドを指定します。

| type       | フィールド例                              | 説明                                     |
|------------|------------------------------------------|------------------------------------------|
| `lever`    | `{ "on": true, "facing": "east" }`      | レバーの初期状態と向き。                   |
| `button`   | `{ "ticks_remaining": 0, "facing": "east" }` | ボタンが押されている残り tick 数と向き。        |
| `dust`     | `{ "power": 0 }`                        | レッドストーンダストの出力レベル (0–15)。 |
| `lamp`     | `{ "on": false }`                       | ランプの点灯状態。                        |
| `repeater` | `{ "delay": 1, "ticks_remaining": 0, "powered": false, "facing": "east" }` | リピータの遅延・向きと現在状態。 |
| `comparator` | `{ "output": 0, "facing": "east" }` | 比較器の出力レベル (0–15) と向き。               |
| `torch`    | `{ "lit": true, "facing": "west" }`    | レッドストーントーチが点灯しているかと取り付け面。    |
| `piston`   | `{ "extended": false, "facing": "up" }` | ピストンが伸びているかどうかと向き。            |
| `hopper`   | `{ "enabled": true, "facing": "down" }` | ホッパーが動作しているかどうかと向き。          |

座標やフィールドの値は整数 (i32) または真偽値です。
`facing` には `north`, `east`, `south`, `west`, `up`, `down` のいずれかを指定します。

## 例
上記の JSON を `simulate_py` に渡すと、レバーをオンにした状態から始まり、隣接するダストを介してランプが点灯するかを確認できます。

```python
import redstonesim

request_json = """{...上記の JSON...}"""
result = redstonesim.simulate_py(request_json)
print(result)
```

`simulate_py` は結果も JSON 文字列として返します。`serde_json` などを用いて `SimResponse` として解釈できます。

## ブロックの接続点を取得する
`block_connections_py` 関数に `PlacedBlock` を表す JSON を渡すと、そのブロックが
どの位置から入力を受け取り、どこへ出力するかを問い合わせられます。

```python
import redstonesim

block_json = '{"x":0,"y":0,"z":0,"type":"dust","power":0}'
info = redstonesim.block_connections_py(block_json)
print(info)  # => {"inputs": [...], "outputs": [...]}
```

結果も JSON 文字列で、`inputs` と `outputs` の配列に各座標が含まれます。

