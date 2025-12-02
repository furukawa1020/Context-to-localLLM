use ifl_core::{IflCore, InputEvent};
use std::thread;
use std::time::Duration;

fn main() {
    // 1. IFL Coreのインスタンスを作成
    let core = IflCore::new();

    // 2. 新しいメッセージセッションを開始
    let session_id = core.start_message();
    println!("Session started: {}", session_id);

    // 3. ユーザーの入力をシミュレーション
    // (実際にはフロントエンドやGUIアプリからイベントを受け取ります)

    let mut current_ts = 1000; // タイムスタンプ (ms)

    // "Hello" とタイプする
    for ch in "Hello".chars() {
        core.push_event(&session_id, InputEvent::KeyInsert { ch, ts: current_ts })
            .unwrap();
        current_ts += 100; // 100msごとに打鍵（普通の速さ）
    }

    // 少し悩む（2秒停止）
    current_ts += 2000;

    // " World" とタイプする
    for ch in " World".chars() {
        core.push_event(&session_id, InputEvent::KeyInsert { ch, ts: current_ts })
            .unwrap();
        current_ts += 100;
    }

    // 送信ボタンを押す
    core.push_event(&session_id, InputEvent::Submit { ts: current_ts })
        .unwrap();

    // 4. セッションを終了して分析結果（プロファイル）を取得
    // 最終的なテキストも渡します
    let final_text = "Hello World";
    let json_result = core.finalize_message(&session_id, final_text).unwrap();

    // 5. 結果を表示
    println!("Analysis Result:\n{}", json_result);
}
