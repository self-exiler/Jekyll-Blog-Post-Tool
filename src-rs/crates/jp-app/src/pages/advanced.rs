//! 高级功能页（对偶 `Pages/AdvancedPage.xaml` + `AdvancedPageViewModel`）。
//!
//! 原版不是缓存页，且**读取配置在 VM 构造函数里 fire-and-forget**——这里同样在 `entered()`
//! 末尾发起 [`settings::load`]，不等它：控件先按空态出现，配置到了再回填。

use std::sync::Arc;

use windows_core::{Param, Result as WinResult};
use winui3::Microsoft::UI::Xaml::Controls::{Page, PasswordBox, StackPanel, TextBox};
use winui3::Microsoft::UI::Xaml::UIElement;

use crate::actions::settings;
use crate::actions::{PageState, Refresh, held};
use crate::services::Services;
use crate::view_models::AiForm;
use crate::widgets;
use super::{PageLifecycle, page_header, page_root, refresh_of, section, state};

const BASE_URL_HINT: &str = "如 https://api.openai.com/v1";
const API_KEY_HINT: &str = "以 Bearer 令牌发送";
const MODEL_HINT: &str = "如 gpt-4o-mini";
const FIELD_MIN_WIDTH: f64 = 320.0;

#[derive(Default)]
pub struct AdvancedPage;

crate::xaml_page!(AdvancedPage);

impl PageLifecycle for AdvancedPage {
    fn entered(&self, base: &Page) -> WinResult<()> {
        let services = crate::services::services();
        let form: PageState<AiForm> = state(AiForm::default());

        let base_url = widgets::text_box("Base URL", "")?;
        let api_key = widgets::password_box("API Key")?;
        let model = widgets::text_box("模型", "")?;

        widgets::min_width(&base_url, FIELD_MIN_WIDTH)?;
        widgets::min_width(&api_key, FIELD_MIN_WIDTH)?;
        widgets::min_width(&model, FIELD_MIN_WIDTH)?;

        let refresh = refresh_of({
            let (base_url, api_key, model) = (base_url.clone(), api_key.clone(), model.clone());
            let form = Arc::clone(&form);
            move || push_to_controls(&form, &base_url, &api_key, &model)
        });

        // 输入实时写回状态：对偶 `UpdateSourceTrigger=PropertyChanged`
        pull(&base_url, &form, |field, value| field.base_url = value)?;
        pull_password(&api_key, &form, |field, value| field.api_key = value)?;
        pull(&model, &form, |field, value| field.model = value)?;

        let content = widgets::vstack(12.0)?;
        widgets::add(&content, &annotated(BASE_URL_HINT, &base_url)?)?;
        widgets::add(&content, &annotated(API_KEY_HINT, &api_key)?)?;
        widgets::add(&content, &annotated(MODEL_HINT, &model)?)?;
        widgets::add(&content, &action_row(&services, &form, &refresh)?)?;

        let body = widgets::vstack(12.0)?;
        widgets::add(
            &body,
            &section(
                "AI 设置",
                "配置 OpenAI 兼容 API，用于在博文页 AI 提取关键字。",
                &content,
            )?,
        )?;

        base.SetContent(&page_root(page_header("高级功能", None)?, &body)?)?;

        settings::load(Arc::clone(&services), Arc::clone(&form), Arc::clone(&refresh));
        Ok(())
    }
}

/// 状态 → 控件。回填会再次触发 `TextChanged`，但写回的是同一个值，不构成回环。
fn push_to_controls(
    form: &PageState<AiForm>,
    base_url: &TextBox,
    api_key: &PasswordBox,
    model: &TextBox,
) {
    let snapshot = held(form).clone();
    let _ = widgets::set_value(base_url, &snapshot.base_url);
    let _ = widgets::set_password(api_key, &snapshot.api_key);
    let _ = widgets::set_value(model, &snapshot.model);
}

/// 控件 → 状态：单行输入框。
fn pull<F>(target: &TextBox, form: &PageState<AiForm>, write: F) -> WinResult<()>
where
    F: Fn(&mut AiForm, String) + Send + 'static,
{
    let control = target.clone();
    let form = Arc::clone(form);
    widgets::on_text_changed(target, move || {
        let value = widgets::value_of(&control);
        write(&mut held(&form), value);
    })
}

/// 控件 → 状态：密码框（`Password` 不参与绑定，只能自己接 `PasswordChanged`）。
fn pull_password<F>(target: &PasswordBox, form: &PageState<AiForm>, write: F) -> WinResult<()>
where
    F: Fn(&mut AiForm, String) + Send + 'static,
{
    let control = target.clone();
    let form = Arc::clone(form);
    widgets::on_password_changed(target, move || {
        let value = widgets::password_of(&control);
        write(&mut held(&form), value);
    })
}

/// 「控件 + 下方灰色说明」的一格表单，对偶 `SettingsCard` 的 Header/Description 排布。
fn annotated<P: Param<UIElement>>(hint: &str, control: P) -> WinResult<StackPanel> {
    let row = widgets::vstack(2.0)?;
    widgets::add(&row, control)?;
    if !hint.is_empty() {
        widgets::add(&row, &widgets::muted_text(hint)?)?;
    }
    Ok(row)
}

fn action_row(services: &Arc<Services>, form: &PageState<AiForm>, refresh: &Refresh) -> WinResult<StackPanel> {
    let row = widgets::hstack(8.0)?;

    let save = widgets::button("保存")?;
    let save_services = Arc::clone(services);
    let save_form = Arc::clone(form);
    let save_refresh = Arc::clone(refresh);
    widgets::on_click(&save, move || {
        settings::save(Arc::clone(&save_services), Arc::clone(&save_form), Arc::clone(&save_refresh));
    })?;

    let clear = widgets::button("清空")?;
    let clear_services = Arc::clone(services);
    let clear_form = Arc::clone(form);
    let clear_refresh = Arc::clone(refresh);
    widgets::on_click(&clear, move || {
        settings::clear(Arc::clone(&clear_services), Arc::clone(&clear_form), Arc::clone(&clear_refresh));
    })?;

    widgets::add(&row, &save)?;
    widgets::add(&row, &clear)?;
    Ok(row)
}
