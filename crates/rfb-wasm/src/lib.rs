// SPDX-License-Identifier: MPL-2.0

use rfb_core::Game;
use rfb_protocol::{
    CharacterSummary, DEMO_CONTENT_HASH, DEMO_CONTENT_ID, GameCommandEnvelope, PROTOCOL_VERSION,
    SaveHeaderV1, from_msgpack, to_msgpack,
};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct WasmGame {
    game: Game,
    created_at: String,
}

#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new(seed: &str, created_at: &str) -> Result<WasmGame, JsValue> {
        let seed = seed
            .parse::<u64>()
            .map_err(|error| js_error(format!("invalid seed: {error}")))?;
        Ok(Self {
            game: Game::new(seed),
            created_at: created_at.to_owned(),
        })
    }

    pub fn snapshot(&self) -> Result<Vec<u8>, JsValue> {
        to_msgpack(&self.game.snapshot()).map_err(js_error)
    }

    pub fn dispatch(&mut self, command: &[u8]) -> Result<Vec<u8>, JsValue> {
        let envelope: GameCommandEnvelope = from_msgpack(command).map_err(js_error)?;
        let update = self.game.dispatch(envelope).map_err(js_error)?;
        to_msgpack(&update).map_err(js_error)
    }

    pub fn save(&self, saved_at: &str) -> Result<Vec<u8>, JsValue> {
        let snapshot = self.game.snapshot();
        let header = SaveHeaderV1 {
            format: "rfb-save".to_owned(),
            save_schema_version: 1,
            game_version: env!("CARGO_PKG_VERSION").to_owned(),
            protocol_version: PROTOCOL_VERSION.to_owned(),
            created_at: self.created_at.clone(),
            saved_at: saved_at.to_owned(),
            character_summary: CharacterSummary {
                display_name: "原创测试探索者".to_owned(),
                level: 1,
                location_key: "location-demo-lab".to_owned(),
                turn: snapshot.turn,
            },
            content_id: DEMO_CONTENT_ID.to_owned(),
            content_hash: DEMO_CONTENT_HASH.to_owned(),
            payload_encoding: "messagepack".to_owned(),
        };
        rfb_save::encode(&header, &self.game.to_save()).map_err(js_error)
    }

    pub fn load(&mut self, bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
        let (header, payload) = rfb_save::decode(bytes).map_err(js_error)?;
        self.game = Game::from_save(payload).map_err(js_error)?;
        self.created_at = header.created_at;
        self.snapshot()
    }
}

fn js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
