# Rust製レッドストーンシミュレータ計画（簡易版）

## 0. レベル定義（L0 / L1 / L2）

- **L0（最小実装）**: レバー／ボタン／ダスト／ランプのON・OFFを処理。減衰なし、強度は二値。
- **L1（基本ブロック網羅）**: ダスト減衰／リピータ／比較器／トーチ／ピストン／ホッパー等を含む。回路設計として主要な構成が可能。
- **L2（発展・バグ技含む）**: Quasi-Connectivity（QC）やゼロティック、BUD、チャンク境界問題など、複雑かつバージョン依存の仕様を含む。

## 1. 概要

- **名称**: RedstoneSim-RS（仮）
- **目的**: Java/Bedrockの回路をローカルで高速シミュレート。テスト・設計補助に活用。
- **対応範囲**: L0–L1相当の基本的なブロック群（初期目標は約50×50ブロック規模）＋一部L2拡張。最終的なワールドサイズ制限は上位レイヤーで制御可能とし、ライブラリ本体では制限を設けない方針。
- **成果物**: Rust製コア、CLI、Pythonバインディング

## 2. 方針

- 段階開発: RustでL0実装→拡張→最適化（PoCからRust一本化）
- 設計: ECS＋スケジューラ、純粋関数＋副作用分離、CIテスト自動化
- 運用: GitHub管理、ライセンスは検討中

## 3. 技術

- Rust 1.79, `bevy_ecs`, `serde`, `glam`
- CLI: `clap`, `rayon`
- FFI: `pyo3`（Pythonバインディング）
- テスト: `criterion`, `quickcheck`
- 試作: Rust にて直接構築

## 4. インターフェース設計（案）

入力:

- ブロックの配置や状態
- 最大ステップ数の指定
- 統合版/Java版の指定とバージョン番号

出力:

- 装置動作過程の状態変更履歴
- 統計

### 入力例

```json
{
  "simulation": {
    "edition": "java",      // "java" または "bedrock"
    "version": "1.21"       // 例: "1.21", "1.20.15"
  },
  "ticks": 5,
  "early_exit": true,
  "world": {
    "blocks": [
      { "pos": [0,0,-1], "tick_at": 0, "id": "redstone_lamp" },
      { "pos": [0,0,0],  "tick_at": 0, "id": "redstone_dust", "meta": { "strength": 0 } },
      { "pos": [0,0,1],  "tick_at": 0, "id": "lever", "meta": { "facing": "north", "powered": false } }
      { "pos": [0,0,1],  "tick_at": 2, "id": "lever", "meta": { "facing": "north", "powered": true } }    // 2tick目にレバーをオンにするユーザー操作に対応したdiff
    ]
  },
}
```

### 出力例

```json
{
  "diffs": [
    {
      "tick": 1,
      "changes": [
        { "x": 0, "y": 0, "z": 0,  "type": "dust", "power": 15 },
        { "x": 0, "y": 0, "z": -1, "type": "lamp", "on": true }
      ]
    }
  ],
  "stats": {
    "ticks_simulated": 2,
    "elapsed_ms": 0.42
  }
}
```

### API 例（Rust）

```rust
pub fn simulate(world: &World, req: &SimRequest) -> SimResponse
```

### CLI コマンド例

```sh
redstonesim run -i world.json -o result.json --ticks 20 --edition java --version 1.21
```

## 5. 次の作業

1. GitHubリポジトリ作成
2. L0 実装用 Rust コードのベース構築
3. Rustワークスペース初期化

