//! AI 配置命令（对偶 `AdvancedPageViewModel` 的三个 RelayCommand）。
//!
//! 表单状态归高级页所有（每次进入新建一份），这里只负责读写 `%APPDATA%\JekyllPostTool\ai.json`
//! 并在结束时把状态推回控件。

use std::sync::Arc;

use jp_application::ai::AiSettings;

use super::{PageState, Refresh, held, notify};
use crate::runtime::spawn_work;
use crate::services::{dialogs, Services};
use crate::view_models::AiForm;

/// 读取 AI 配置回填表单（对偶构造里的 fire-and-forget `LoadAiSettingsAsync`）。
pub fn load(services: Arc<Services>, state: PageState<AiForm>, refresh: Refresh) {
    spawn_work(
        move || match services.ai_settings.get() {
            Ok(settings) => held(&state).apply_settings(&settings),
            // .NET 侧读失败只落 Debug 输出，这里按动作层约定弹一次
            Err(error) => dialogs::info("读取 AI 配置失败", &error.to_string()),
        },
        move |_: ()| notify(&refresh),
    )
}

/// 保存 AI 配置（对偶 `SaveAiSettingsAsync`）。
pub fn save(services: Arc<Services>, state: PageState<AiForm>, refresh: Refresh) {
    spawn_work(
        move || {
            // 先出锁再落盘：克隆一份配置，IO 期间不持有表单状态
            let settings = held(&state).to_settings();

            match services.ai_settings.set(&settings) {
                Ok(()) => dialogs::info("已保存", "AI 配置已保存。"),
                // .NET 侧这个异常没人接（fire-and-forget），这里不静默
                Err(error) => dialogs::info("保存失败", &error.to_string()),
            }
        },
        move |_: ()| notify(&refresh),
    )
}

/// 清空 AI 配置（对偶 `ClearAiSettingsAsync`）：表单与磁盘一起清空，成功时不弹提示。
pub fn clear(services: Arc<Services>, state: PageState<AiForm>, refresh: Refresh) {
    spawn_work(
        move || {
            held(&state).clear();

            if let Err(error) = services.ai_settings.set(&AiSettings::default()) {
                dialogs::info("清除失败", &error.to_string());
            }
        },
        move |_: ()| notify(&refresh),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[test]
    fn saving_persisted_settings_round_trip_through_the_form() {
        // 动作层只依赖 `to_settings` 的 trim 语义：这里锁住「表单 → 落盘对象」这一跳
        let state: PageState<AiForm> = Arc::new(Mutex::new(AiForm {
            base_url: " https://api.test/v1 ".to_owned(),
            api_key: " sk-test ".to_owned(),
            model: " gpt-4o-mini ".to_owned(),
        }));

        let settings = held(&state).to_settings();
        assert!(settings.is_configured());

        held(&state).apply_settings(&settings);
        assert_eq!(&held(&state).base_url, "https://api.test/v1");
    }

    #[test]
    fn clearing_resets_the_form_to_empty_settings() {
        let state: PageState<AiForm> = Arc::new(Mutex::new(AiForm {
            base_url: "https://api.test/v1".to_owned(),
            api_key: "sk-test".to_owned(),
            model: "gpt-4o-mini".to_owned(),
        }));

        held(&state).clear();

        assert_eq!(held(&state).to_settings(), AiSettings::default());
        assert!(!held(&state).to_settings().is_configured());
    }
}
