use std::str::FromStr;

use crate::amll_connector::WebsocketStatus;
use crate::app_definition::{
    AppView, BatchConverterStatus, PreviewState, SearchState, UniLyricApp,
};

use crate::app_settings::AppAmllMirror;
use crate::types::{AutoSearchSource, AutoSearchStatus};

use crate::app_actions::{
    AmllConnectorAction, DownloaderAction, LyricsAction, PanelType, PlayerAction, ProcessorType,
    SettingsAction, UIAction, UserAction,
};
use eframe::egui::{self, Align, Button, ComboBox, Layout, ScrollArea, Spinner, TextEdit};
use egui::Color32;
use log::LevelFilter;
use lyrics_helper_core::{CanonicalMetadataKey, ChineseConversionConfig, FullLyricsResult};

const TITLE_ALIGNMENT_OFFSET: f32 = 6.0;
const BUTTON_STRIP_SPACING: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsCategory {
    #[default]
    General,
    Interface,
    AutoSearch,
    Connector,
    Postprocessors,
}

impl SettingsCategory {
    fn display_name(&self) -> &'static str {
        match self {
            SettingsCategory::General => "通用",
            SettingsCategory::Interface => "界面",
            SettingsCategory::AutoSearch => "自动搜索",
            SettingsCategory::Connector => "AMLL Connector",
            SettingsCategory::Postprocessors => "后处理器",
        }
    }
}

impl UniLyricApp {
    pub fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui_bar| {
            ui_bar.menu_button("文件", |file_menu| {
                if file_menu
                    .add(egui::Button::new("打开歌词文件..."))
                    .clicked()
                {
                    self.send_action(crate::app_actions::UserAction::File(
                        crate::app_actions::FileAction::Open,
                    ));
                }
                file_menu.separator();
                let main_lyrics_loaded = (self.lyrics.parsed_lyric_data.is_some()
                    && self.lyrics.parsed_lyric_data.as_ref().is_some())
                    || !self.lyrics.input_text.is_empty();
                let lrc_load_enabled = main_lyrics_loaded && !self.lyrics.conversion_in_progress;
                let disabled_lrc_hover_text = "请先加载主歌词文件或内容";

                let translation_button = egui::Button::new("加载翻译 (LRC)...");
                let mut translation_button_response =
                    file_menu.add_enabled(lrc_load_enabled, translation_button);
                if !lrc_load_enabled {
                    translation_button_response =
                        translation_button_response.on_disabled_hover_text(disabled_lrc_hover_text);
                }
                if translation_button_response.clicked() {
                    self.send_action(crate::app_actions::UserAction::File(
                        crate::app_actions::FileAction::LoadTranslationLrc,
                    ));
                }

                let romanization_button = egui::Button::new("加载罗马音 (LRC)...");
                let mut romanization_button_response =
                    file_menu.add_enabled(lrc_load_enabled, romanization_button);
                if !lrc_load_enabled {
                    romanization_button_response = romanization_button_response
                        .on_disabled_hover_text(disabled_lrc_hover_text);
                }
                if romanization_button_response.clicked() {
                    self.send_action(crate::app_actions::UserAction::File(
                        crate::app_actions::FileAction::LoadRomanizationLrc,
                    ));
                }
                file_menu.separator();

                file_menu.menu_button("下载歌词...", |download_menu| {
                    if download_menu
                        .add(egui::Button::new("搜索歌词..."))
                        .clicked()
                    {
                        self.send_action(crate::app_actions::UserAction::UI(
                            crate::app_actions::UIAction::SetView(
                                crate::app_definition::AppView::Downloader,
                            ),
                        ));
                    }
                });

                file_menu.menu_button("批量处理...", |batch_menu| {
                    if batch_menu.button("批量转换...").clicked() {
                        self.send_action(UserAction::UI(UIAction::SetView(
                            AppView::BatchConverter,
                        )));
                    }
                });

                file_menu.separator();
                if file_menu
                    .add_enabled(
                        !self.lyrics.output_text.is_empty(),
                        egui::Button::new("保存输出为..."),
                    )
                    .clicked()
                {
                    self.send_action(crate::app_actions::UserAction::File(
                        crate::app_actions::FileAction::Save,
                    ));
                }
            });

            ui_bar.menu_button("后处理", |postprocess_menu| {
                let lyrics_loaded = self.lyrics.parsed_lyric_data.is_some();

                if postprocess_menu
                    .add_enabled(lyrics_loaded, egui::Button::new("清理元数据行"))
                    .on_disabled_hover_text("需要先成功解析歌词")
                    .clicked()
                {
                    self.send_action(UserAction::Lyrics(Box::new(LyricsAction::ApplyProcessor(
                        ProcessorType::MetadataStripper,
                    ))));
                }

                if postprocess_menu
                    .add_enabled(lyrics_loaded, egui::Button::new("音节平滑"))
                    .on_disabled_hover_text("需要先成功解析歌词")
                    .clicked()
                {
                    self.send_action(UserAction::Lyrics(Box::new(LyricsAction::ApplyProcessor(
                        ProcessorType::SyllableSmoother,
                    ))));
                }

                if postprocess_menu
                    .add_enabled(lyrics_loaded, egui::Button::new("演唱者识别"))
                    .on_disabled_hover_text("需要先成功解析歌词")
                    .clicked()
                {
                    self.send_action(UserAction::Lyrics(Box::new(LyricsAction::ApplyProcessor(
                        ProcessorType::AgentRecognizer,
                    ))));
                }
            });

            ui_bar.menu_button("简繁转换", |tools_menu| {
                let conversion_enabled = !self.lyrics.input_text.is_empty()
                    || self
                        .lyrics
                        .parsed_lyric_data
                        .as_ref()
                        .is_some_and(|d| !d.lines.is_empty());

                tools_menu.label(egui::RichText::new("通用简繁转换").strong());
                self.draw_chinese_conversion_menu_item(
                    tools_menu,
                    ChineseConversionConfig::S2t,
                    "简体 → 繁体 (通用)",
                    conversion_enabled,
                );
                self.draw_chinese_conversion_menu_item(
                    tools_menu,
                    ChineseConversionConfig::T2s,
                    "繁体 → 简体 (通用)",
                    conversion_enabled,
                );
                tools_menu.separator();

                tools_menu.label(egui::RichText::new("地区性转换 (含用语)").strong());
                tools_menu.menu_button("简体 →", |sub_menu| {
                    self.draw_chinese_conversion_menu_item(
                        sub_menu,
                        ChineseConversionConfig::S2twp,
                        "台湾正体",
                        conversion_enabled,
                    );
                    self.draw_chinese_conversion_menu_item(
                        sub_menu,
                        ChineseConversionConfig::S2hk,
                        "香港繁体",
                        conversion_enabled,
                    );
                });
                tools_menu.menu_button("繁体 →", |sub_menu| {
                    self.draw_chinese_conversion_menu_item(
                        sub_menu,
                        ChineseConversionConfig::Tw2sp,
                        "大陆简体 (含用语)",
                        conversion_enabled,
                    );
                    self.draw_chinese_conversion_menu_item(
                        sub_menu,
                        ChineseConversionConfig::Tw2s,
                        "大陆简体 (仅文字)",
                        conversion_enabled,
                    );
                });
                tools_menu.separator();

                tools_menu.label(egui::RichText::new("仅文字转换").strong());
                tools_menu.menu_button("繁体互转", |sub_menu| {
                    self.draw_chinese_conversion_menu_item(
                        sub_menu,
                        ChineseConversionConfig::Tw2t,
                        "台湾繁体 → 香港繁体",
                        conversion_enabled,
                    );
                    self.draw_chinese_conversion_menu_item(
                        sub_menu,
                        ChineseConversionConfig::Hk2t,
                        "香港繁体 → 台湾繁体",
                        conversion_enabled,
                    );
                });
                tools_menu.menu_button("其他转换", |sub_menu| {
                    self.draw_chinese_conversion_menu_item(
                        sub_menu,
                        ChineseConversionConfig::S2tw,
                        "简体 → 台湾繁体 (仅文字)",
                        conversion_enabled,
                    );
                    self.draw_chinese_conversion_menu_item(
                        sub_menu,
                        ChineseConversionConfig::T2tw,
                        "繁体 → 台湾繁体 (异体字)",
                        conversion_enabled,
                    );
                    self.draw_chinese_conversion_menu_item(
                        sub_menu,
                        ChineseConversionConfig::T2hk,
                        "繁体 → 香港繁体 (异体字)",
                        conversion_enabled,
                    );
                    self.draw_chinese_conversion_menu_item(
                        sub_menu,
                        ChineseConversionConfig::Hk2s,
                        "香港繁体 → 简体",
                        conversion_enabled,
                    );
                });
                tools_menu.separator();

                tools_menu.label(egui::RichText::new("日语汉字转换").strong());
                self.draw_chinese_conversion_menu_item(
                    tools_menu,
                    ChineseConversionConfig::Jp2t,
                    "日语新字体 → 繁体旧字体",
                    conversion_enabled,
                );
                self.draw_chinese_conversion_menu_item(
                    tools_menu,
                    ChineseConversionConfig::T2jp,
                    "繁体旧字体 → 日语新字体",
                    conversion_enabled,
                );
            });

            ui_bar.add_space(16.0);
            ui_bar.label("源格式:");
            let mut temp_source_format = self.lyrics.source_format;

            egui::ComboBox::from_id_salt("source_format_toolbar")
                .selected_text(self.lyrics.source_format.to_string())
                .show_ui(ui_bar, |ui_combo| {
                    for fmt_option in &self.lyrics.available_formats {
                        let display_text = fmt_option.to_string();
                        let is_selectable_source = true;

                        let response = ui_combo
                            .add_enabled_ui(is_selectable_source, |ui_selectable| {
                                ui_selectable.selectable_value(
                                    &mut temp_source_format,
                                    *fmt_option,
                                    display_text,
                                )
                            })
                            .inner;

                        if response.clicked() && is_selectable_source {
                            ui_combo.close_menu();
                        }
                    }
                });

            if temp_source_format != self.lyrics.source_format {
                self.send_action(crate::app_actions::UserAction::Lyrics(Box::new(
                    crate::app_actions::LyricsAction::SourceFormatChanged(temp_source_format),
                )));
            }

            ui_bar.add_space(8.0);
            ui_bar.label("目标格式:");
            let mut _target_format_changed_this_frame = false;
            let mut temp_target_format = self.lyrics.target_format;

            egui::ComboBox::from_id_salt("target_format_toolbar")
                .selected_text(self.lyrics.target_format.to_string())
                .show_ui(ui_bar, |ui_combo| {
                    for fmt_option in &self.lyrics.available_formats {
                        let display_text = fmt_option.to_string();
                        if ui_combo
                            .selectable_value(&mut temp_target_format, *fmt_option, display_text)
                            .clicked()
                        {
                            ui_combo.close_menu();
                        }
                    }
                });

            if temp_target_format != self.lyrics.target_format {
                self.send_action(crate::app_actions::UserAction::Lyrics(Box::new(
                    crate::app_actions::LyricsAction::TargetFormatChanged(temp_target_format),
                )));
            }

            ui_bar.with_layout(Layout::right_to_left(Align::Center), |ui_right| {
                ui_right.menu_button("视图", |view_menu| {
                    let mut show_translation_lrc_panel_copy = self.ui.show_translation_lrc_panel;
                    if view_menu
                        .checkbox(&mut show_translation_lrc_panel_copy, "翻译LRC面板")
                        .changed()
                    {
                        self.send_action(crate::app_actions::UserAction::UI(
                            crate::app_actions::UIAction::SetPanelVisibility(
                                crate::app_actions::PanelType::Translation,
                                show_translation_lrc_panel_copy,
                            ),
                        ));
                    }

                    let mut show_romanization_lrc_panel_copy = self.ui.show_romanization_lrc_panel;
                    if view_menu
                        .checkbox(&mut show_romanization_lrc_panel_copy, "罗马音LRC面板")
                        .changed()
                    {
                        self.send_action(crate::app_actions::UserAction::UI(
                            crate::app_actions::UIAction::SetPanelVisibility(
                                crate::app_actions::PanelType::Romanization,
                                show_romanization_lrc_panel_copy,
                            ),
                        ));
                    }

                    view_menu.separator();

                    let amll_connector_feature_enabled =
                        self.amll_connector.config.lock().unwrap().enabled;
                    view_menu
                        .add_enabled_ui(amll_connector_feature_enabled, |ui_enabled_check| {
                            let mut show_amll_sidebar_copy = self.ui.show_amll_connector_sidebar;
                            if ui_enabled_check
                                .checkbox(&mut show_amll_sidebar_copy, "AMLL Connector 侧边栏")
                                .changed()
                            {
                                self.send_action(crate::app_actions::UserAction::UI(
                                    crate::app_actions::UIAction::SetPanelVisibility(
                                        crate::app_actions::PanelType::AmllConnector,
                                        show_amll_sidebar_copy,
                                    ),
                                ));
                            }
                        })
                        .response
                        .on_disabled_hover_text("请在设置中启用 AMLL Connector 功能");

                    view_menu.separator();

                    let mut show_log_panel_copy = self.ui.show_bottom_log_panel;
                    if view_menu
                        .checkbox(&mut show_log_panel_copy, "日志面板")
                        .changed()
                    {
                        self.send_action(crate::app_actions::UserAction::UI(
                            crate::app_actions::UIAction::SetPanelVisibility(
                                crate::app_actions::PanelType::Log,
                                show_log_panel_copy,
                            ),
                        ));
                    }
                });
                ui_right.add_space(BUTTON_STRIP_SPACING);
                if ui_right.button("元数据").clicked() {
                    self.send_action(crate::app_actions::UserAction::UI(
                        crate::app_actions::UIAction::ShowPanel(
                            crate::app_actions::PanelType::Metadata,
                        ),
                    ));
                }
                ui_right.add_space(BUTTON_STRIP_SPACING);
                let mut wrap_text_copy = self.ui.wrap_text;
                if ui_right.checkbox(&mut wrap_text_copy, "自动换行").changed() {
                    self.send_action(crate::app_actions::UserAction::UI(
                        crate::app_actions::UIAction::SetWrapText(wrap_text_copy),
                    ));
                }
                ui_right.add_space(BUTTON_STRIP_SPACING);
                if ui_right.button("设置").clicked() {
                    self.send_action(crate::app_actions::UserAction::UI(
                        crate::app_actions::UIAction::ShowPanel(
                            crate::app_actions::PanelType::Settings,
                        ),
                    ));
                }
            });
        });
    }

    /// 绘制应用设置窗口。
    pub fn draw_settings_window(&mut self, ctx: &egui::Context) {
        let mut is_settings_window_open = self.ui.show_settings_window;

        egui::Window::new("应用程序设置")
            .open(&mut is_settings_window_open)
            .resizable(true)
            .default_width(700.0)
            .max_height(450.0)
            .show(ctx, |ui| {
                ui.horizontal_top(|h_ui| {
                    egui::SidePanel::left("settings_category_panel")
                        .exact_width(140.0)
                        .show_inside(h_ui, |nav_ui| {
                            nav_ui.style_mut().spacing.item_spacing = egui::vec2(4.0, 8.0);
                            nav_ui.heading("设置");
                            nav_ui.separator();

                            let categories = [
                                SettingsCategory::General,
                                SettingsCategory::Interface,
                                SettingsCategory::AutoSearch,
                                SettingsCategory::Connector,
                                SettingsCategory::Postprocessors,
                            ];

                            for category in categories {
                                nav_ui.selectable_value(
                                    &mut self.ui.current_settings_category,
                                    category,
                                    category.display_name(),
                                );
                            }
                        });

                    egui::CentralPanel::default().show_inside(h_ui, |content_ui| {
                        ScrollArea::vertical().show(content_ui, |scroll_ui| {
                            match self.ui.current_settings_category {
                                SettingsCategory::General => self.draw_settings_general(scroll_ui),
                                SettingsCategory::Interface => {
                                    self.draw_settings_interface(scroll_ui)
                                }
                                SettingsCategory::AutoSearch => {
                                    self.draw_settings_auto_search(scroll_ui)
                                }
                                SettingsCategory::Connector => {
                                    self.draw_settings_amll_connector(scroll_ui)
                                }
                                SettingsCategory::Postprocessors => {
                                    self.draw_settings_postprocessors(scroll_ui)
                                }
                            }
                        });
                    });
                });
                ui.separator();
                ui.with_layout(
                    Layout::right_to_left(egui::Align::Center),
                    |bottom_buttons_ui| {
                        if bottom_buttons_ui.button("取消").clicked() {
                            self.send_action(crate::app_actions::UserAction::Settings(
                                crate::app_actions::SettingsAction::Cancel,
                            ));
                        }
                        if bottom_buttons_ui
                            .button("重置")
                            .on_hover_text("撤销当前窗口中的所有更改")
                            .clicked()
                        {
                            self.send_action(UserAction::Settings(SettingsAction::Reset));
                        }
                        if bottom_buttons_ui
                            .button("保存并应用")
                            .on_hover_text(
                                "保存设置到文件。部分设置将在下次启动或下次自动搜索时生效",
                            )
                            .clicked()
                        {
                            self.send_action(crate::app_actions::UserAction::Settings(
                                crate::app_actions::SettingsAction::Save(Box::new(
                                    self.ui.temp_edit_settings.clone(),
                                )),
                            ));
                        }
                    },
                );
            });

        if !is_settings_window_open {
            self.ui.show_settings_window = false;
        }
    }

    fn draw_settings_general(&mut self, ui: &mut egui::Ui) {
        ui.heading("通用设置");
        ui.add_space(10.0);

        egui::Grid::new("log_settings_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |grid_ui| {
                grid_ui.label("启用文件日志:");
                grid_ui.checkbox(
                    &mut self.ui.temp_edit_settings.log_settings.enable_file_log,
                    "",
                );
                grid_ui.end_row();

                grid_ui.label("文件日志级别:");
                ComboBox::from_id_salt("file_log_level_combo_settings")
                    .selected_text(format!(
                        "{:?}",
                        self.ui.temp_edit_settings.log_settings.file_log_level
                    ))
                    .show_ui(grid_ui, |ui_combo| {
                        ui_combo.selectable_value(
                            &mut self.ui.temp_edit_settings.log_settings.file_log_level,
                            LevelFilter::Off,
                            "Off",
                        );
                        ui_combo.selectable_value(
                            &mut self.ui.temp_edit_settings.log_settings.file_log_level,
                            LevelFilter::Error,
                            "Error",
                        );
                        ui_combo.selectable_value(
                            &mut self.ui.temp_edit_settings.log_settings.file_log_level,
                            LevelFilter::Warn,
                            "Warn",
                        );
                        ui_combo.selectable_value(
                            &mut self.ui.temp_edit_settings.log_settings.file_log_level,
                            LevelFilter::Info,
                            "Info",
                        );
                        ui_combo.selectable_value(
                            &mut self.ui.temp_edit_settings.log_settings.file_log_level,
                            LevelFilter::Debug,
                            "Debug",
                        );
                        ui_combo.selectable_value(
                            &mut self.ui.temp_edit_settings.log_settings.file_log_level,
                            LevelFilter::Trace,
                            "Trace",
                        );
                    });
                grid_ui.end_row();

                grid_ui.label("控制台日志级别:");
                ComboBox::from_id_salt("console_log_level_combo_settings")
                    .selected_text(format!(
                        "{:?}",
                        self.ui.temp_edit_settings.log_settings.console_log_level
                    ))
                    .show_ui(grid_ui, |ui_combo| {
                        ui_combo.selectable_value(
                            &mut self.ui.temp_edit_settings.log_settings.console_log_level,
                            LevelFilter::Off,
                            "Off",
                        );
                        ui_combo.selectable_value(
                            &mut self.ui.temp_edit_settings.log_settings.console_log_level,
                            LevelFilter::Error,
                            "Error",
                        );
                        ui_combo.selectable_value(
                            &mut self.ui.temp_edit_settings.log_settings.console_log_level,
                            LevelFilter::Warn,
                            "Warn",
                        );
                        ui_combo.selectable_value(
                            &mut self.ui.temp_edit_settings.log_settings.console_log_level,
                            LevelFilter::Info,
                            "Info",
                        );
                        ui_combo.selectable_value(
                            &mut self.ui.temp_edit_settings.log_settings.console_log_level,
                            LevelFilter::Debug,
                            "Debug",
                        );
                        ui_combo.selectable_value(
                            &mut self.ui.temp_edit_settings.log_settings.console_log_level,
                            LevelFilter::Trace,
                            "Trace",
                        );
                    });
                grid_ui.end_row();
            });
    }

    fn draw_settings_interface(&mut self, ui: &mut egui::Ui) {
        ui.heading("界面设置");
        ui.add_space(10.0);

        ui.horizontal(|h_ui| {
            h_ui.label("界面字体:");

            let mut selected = self
                .ui
                .temp_edit_settings
                .selected_font_family
                .clone()
                .unwrap_or_else(|| "默认".to_string());

            egui::ComboBox::from_label("")
                .selected_text(&selected)
                .show_ui(h_ui, |combo_ui| {
                    if combo_ui
                        .selectable_value(&mut selected, "默认".to_string(), "默认 (内置字体)")
                        .clicked()
                    {
                        self.ui.temp_edit_settings.selected_font_family = None;
                    }
                    for font_name in &self.ui.available_system_fonts {
                        if combo_ui
                            .selectable_value(&mut selected, font_name.clone(), font_name)
                            .clicked()
                        {
                            self.ui.temp_edit_settings.selected_font_family =
                                Some(font_name.clone());
                        }
                    }
                });
        });
    }

    fn draw_settings_auto_search(&mut self, ui: &mut egui::Ui) {
        ui.heading("自动歌词搜索设置");
        ui.add_space(10.0);

        let auto_cache_enabled = self.ui.temp_edit_settings.auto_cache;

        ui.checkbox(&mut self.ui.temp_edit_settings.auto_cache, "自动缓存歌词");

        ui.add_enabled_ui(auto_cache_enabled, |enabled_ui| {
            enabled_ui.horizontal(|h_ui| {
                h_ui.label("最多缓存数量:");
                h_ui.add(
                    egui::DragValue::new(&mut self.ui.temp_edit_settings.auto_cache_max_count)
                        .speed(1.0),
                );
            });
        });

        ui.separator();
        ui.checkbox(
            &mut self.ui.temp_edit_settings.prioritize_amll_db,
            "优先搜索 AMLL TTML 数据库 (推荐)",
        );
        ui.checkbox(
            &mut self.ui.temp_edit_settings.enable_t2s_for_auto_search,
            "将繁体 SMTC 信息转为简体再搜索 (推荐)",
        );
        ui.checkbox(
            &mut self.ui.temp_edit_settings.always_search_all_sources,
            "始终搜索所有源 (推荐)",
        );
        ui.add_space(10.0);
        ui.checkbox(
            &mut self.ui.temp_edit_settings.use_provider_subset,
            "只在以下选择的源中搜索:",
        );

        ui.add_enabled_ui(
            self.ui.temp_edit_settings.use_provider_subset,
            |enabled_ui| {
                egui::Frame::group(enabled_ui.style()).show(enabled_ui, |group_ui| {
                    group_ui.label("选择要使用的提供商:");
                    let all_providers = AutoSearchSource::default_order();
                    for provider in all_providers {
                        let provider_name = Into::<&'static str>::into(provider).to_string();
                        let mut is_selected = self
                            .ui
                            .temp_edit_settings
                            .auto_search_provider_subset
                            .contains(&provider_name);
                        if group_ui
                            .checkbox(&mut is_selected, provider.display_name())
                            .changed()
                        {
                            if is_selected {
                                self.ui
                                    .temp_edit_settings
                                    .auto_search_provider_subset
                                    .push(provider_name);
                            } else {
                                self.ui
                                    .temp_edit_settings
                                    .auto_search_provider_subset
                                    .retain(|p| p != &provider_name);
                            }
                        }
                    }
                });
            },
        );
    }

    fn draw_settings_amll_connector(&mut self, ui: &mut egui::Ui) {
        ui.heading("AMLL Connector 设置");
        ui.add_space(10.0);

        egui::Grid::new("amll_connector_settings_grid")
            .num_columns(2)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |grid_ui| {
                grid_ui.label("启用 AMLL Connector 功能:");
                grid_ui
                    .checkbox(&mut self.ui.temp_edit_settings.amll_connector_enabled, "")
                    .on_hover_text(
                        "转发 SMTC 信息到 AMLL Player，让 AMLL Player 也支持其他音乐软件",
                    );
                grid_ui.end_row();

                grid_ui.label("WebSocket URL:");
                grid_ui
                    .add(
                        TextEdit::singleline(
                            &mut self.ui.temp_edit_settings.amll_connector_websocket_url,
                        )
                        .hint_text("ws://localhost:11444")
                        .desired_width(f32::INFINITY),
                    )
                    .on_hover_text("需点击“保存并应用”");
                grid_ui.end_row();

                grid_ui.label("将音频数据发送到 AMLL Player");
                grid_ui.checkbox(
                    &mut self.ui.temp_edit_settings.send_audio_data_to_player,
                    "",
                );
                grid_ui.end_row();

                grid_ui
                    .label("时间轴偏移量 (毫秒):")
                    .on_hover_text("调整SMTC报告的时间戳以匹配歌词");
                grid_ui.add(
                    egui::DragValue::new(&mut self.ui.temp_edit_settings.smtc_time_offset_ms)
                        .speed(10.0)
                        .suffix(" ms"),
                );
                grid_ui.end_row();

                grid_ui
                    .label("校准时间轴")
                    .on_hover_text("切歌时立刻跳转到0ms，可能对 Spotify 有奇效");
                grid_ui.checkbox(
                    &mut self.ui.temp_edit_settings.calibrate_timeline_on_song_change,
                    "",
                );
                grid_ui.end_row();

                grid_ui
                    .label("在新曲目开始时快速暂停/播放")
                    .on_hover_text("更强力地校准时间轴");
                grid_ui.checkbox(
                    &mut self.ui.temp_edit_settings.flicker_play_pause_on_song_change,
                    "",
                );
                grid_ui.end_row();
            });
        ui.add_space(10.0);
        ui.strong("AMLL DB 镜像");

        ui.horizontal(|h_ui| {
            if h_ui.button("立即检查更新").clicked() {
                self.send_action(UserAction::AmllConnector(
                    AmllConnectorAction::CheckIndexUpdate,
                ));
            }

            if h_ui.button("重新加载所有提供商").clicked() {
                self.send_action(UserAction::AmllConnector(
                    AmllConnectorAction::ReloadProviders,
                ));
            }
        });

        let current_mirror = &mut self.ui.temp_edit_settings.amll_mirror;

        let mirror_name = match current_mirror {
            AppAmllMirror::GitHub => "GitHub",
            AppAmllMirror::Dimeta => "Dimeta",
            AppAmllMirror::Bikonoo => "Bikonoo",
            AppAmllMirror::Custom { .. } => "自定义",
        };

        ComboBox::from_id_salt("amll_mirror_selector")
            .selected_text(mirror_name)
            .show_ui(ui, |combo_ui| {
                combo_ui.selectable_value(current_mirror, AppAmllMirror::Dimeta, "Dimeta");
                combo_ui.selectable_value(current_mirror, AppAmllMirror::Bikonoo, "Bikonoo");
                combo_ui.selectable_value(current_mirror, AppAmllMirror::GitHub, "GitHub (主源)");

                let is_custom = matches!(current_mirror, AppAmllMirror::Custom { .. });
                if combo_ui.selectable_label(is_custom, "自定义").clicked() && !is_custom {
                    *current_mirror = AppAmllMirror::Custom {
                        index_url: String::new(),
                        lyrics_url_template: String::new(),
                    };
                }
            });

        if let AppAmllMirror::Custom {
            index_url,
            lyrics_url_template,
        } = current_mirror
        {
            ui.add_space(5.0);
            ui.label("索引 URL:");
            ui.text_edit_singleline(index_url)
                .on_hover_text("指向 raw-lyrics-index.jsonl 文件的完整 URL");

            ui.label("歌词模板 URL:");
            ui.text_edit_singleline(lyrics_url_template).on_hover_text(
                "必须包含 {song_id} 占位符，例如：https://my.mirror/lyrics/{song_id}",
            );
        }
    }

    fn draw_settings_postprocessors(&mut self, ui: &mut egui::Ui) {
        ui.heading("后处理器设置");
        ui.separator();

        ui.strong("自动应用");
        ui.label("自动获取歌词后，运行以下后处理器：");
        ui.checkbox(
            &mut self.ui.temp_edit_settings.auto_apply_metadata_stripper,
            "清理元数据行",
        );
        ui.checkbox(
            &mut self.ui.temp_edit_settings.auto_apply_agent_recognizer,
            "识别演唱者",
        );
        ui.separator();

        ui.collapsing("元数据清理器", |stripper_ui| {
            let options = &mut self.ui.temp_edit_settings.metadata_stripper;

            let mut is_enabled = options
                .flags
                .contains(lyrics_helper_core::MetadataStripperFlags::ENABLED);
            if stripper_ui
                .checkbox(&mut is_enabled, "启用元数据清理")
                .changed()
            {
                options.flags.set(
                    lyrics_helper_core::MetadataStripperFlags::ENABLED,
                    is_enabled,
                );
            }

            let mut keyword_case_sensitive = options
                .flags
                .contains(lyrics_helper_core::MetadataStripperFlags::KEYWORD_CASE_SENSITIVE);
            if stripper_ui
                .checkbox(&mut keyword_case_sensitive, "关键词匹配区分大小写")
                .changed()
            {
                options.flags.set(
                    lyrics_helper_core::MetadataStripperFlags::KEYWORD_CASE_SENSITIVE,
                    keyword_case_sensitive,
                );
            }

            let mut regex_enabled = options
                .flags
                .contains(lyrics_helper_core::MetadataStripperFlags::ENABLE_REGEX_STRIPPING);
            if stripper_ui
                .checkbox(&mut regex_enabled, "启用正则表达式清理")
                .changed()
            {
                options.flags.set(
                    lyrics_helper_core::MetadataStripperFlags::ENABLE_REGEX_STRIPPING,
                    regex_enabled,
                );
            }

            let mut regex_case_sensitive = options
                .flags
                .contains(lyrics_helper_core::MetadataStripperFlags::REGEX_CASE_SENSITIVE);
            if stripper_ui
                .checkbox(&mut regex_case_sensitive, "正则表达式匹配区分大小写")
                .changed()
            {
                options.flags.set(
                    lyrics_helper_core::MetadataStripperFlags::REGEX_CASE_SENSITIVE,
                    regex_case_sensitive,
                );
            }

            stripper_ui.label("关键词 (每行一个):");
            let mut keywords_text = options.keywords.join("\n");
            if stripper_ui
                .add(TextEdit::multiline(&mut keywords_text).desired_rows(3))
                .changed()
            {
                options.keywords = keywords_text.lines().map(String::from).collect();
            }

            stripper_ui.label("正则表达式 (每行一个):");
            let mut regex_text = options.regex_patterns.join("\n");
            if stripper_ui
                .add(TextEdit::multiline(&mut regex_text).desired_rows(3))
                .changed()
            {
                options.regex_patterns = regex_text.lines().map(String::from).collect();
            }
        });

        ui.collapsing("音节平滑", |smoothing_ui| {
            let options = &mut self.ui.temp_edit_settings.syllable_smoothing;

            smoothing_ui.horizontal(|h_ui| {
                h_ui.label("平滑因子 (0.0-0.5):");
                h_ui.add(egui::Slider::new(&mut options.factor, 0.0..=0.5));
            });
            smoothing_ui.horizontal(|h_ui| {
                h_ui.label("平滑迭代次数:");
                h_ui.add(egui::DragValue::new(&mut options.smoothing_iterations).speed(1.0));
            });
            smoothing_ui.horizontal(|h_ui| {
                h_ui.label("时长差异阈值 (ms):");
                h_ui.add(egui::DragValue::new(&mut options.duration_threshold_ms).speed(1.0));
            });
            smoothing_ui.horizontal(|h_ui| {
                h_ui.label("间隔阈值 (ms):");
                h_ui.add(egui::DragValue::new(&mut options.gap_threshold_ms).speed(1.0));
            });
        });
    }

    pub fn draw_metadata_editor_window_contents(&mut self, ui: &mut egui::Ui, _open: &mut bool) {
        let mut actions_to_send = Vec::new();

        egui::ScrollArea::vertical().show(ui, |scroll_ui| {
            if self.lyrics.metadata_manager.ui_entries.is_empty() {
                scroll_ui.label(
                    egui::RichText::new("无元数据可编辑。\n可从文件加载，或手动添加。").weak(),
                );
                return;
            }

            let mut deletion_index: Option<usize> = None;
            let mut previous_key: Option<&CanonicalMetadataKey> = None;

            for (index, entry) in self
                .lyrics
                .metadata_manager
                .ui_entries
                .iter_mut()
                .enumerate()
            {
                let item_id = entry.id;
                let is_first_in_group = previous_key != Some(&entry.key);
                if is_first_in_group && index > 0 {
                    scroll_ui.separator();
                }
                scroll_ui.horizontal(|row_ui| {
                    if row_ui.checkbox(&mut entry.is_pinned, "").changed() {
                        actions_to_send.push(UserAction::Lyrics(Box::new(
                            LyricsAction::ToggleMetadataPinned(index),
                        )));
                    }
                    row_ui
                        .label("固定")
                        .on_hover_text("勾选后, 此条元数据在加载新歌词时将尝试保留其值");

                    let key_editor_width = row_ui.available_width() * 0.3;
                    let mut key_changed_this_frame = false;

                    if is_first_in_group {
                        row_ui.add_space(5.0);
                        row_ui.label("键:");
                        if let CanonicalMetadataKey::Custom(custom_key_str) = &mut entry.key {
                            let response = row_ui.add_sized(
                                [key_editor_width, 0.0],
                                egui::TextEdit::singleline(custom_key_str)
                                    .id_salt(item_id.with("key_edit_custom")),
                            );
                            if response.lost_focus() && response.changed() {
                                if let Ok(parsed_key) =
                                    CanonicalMetadataKey::from_str(custom_key_str)
                                {
                                    entry.key = parsed_key;
                                }
                                key_changed_this_frame = true;
                            }
                        } else {
                            egui::ComboBox::from_id_salt(item_id.with("key_combo"))
                                .selected_text(entry.key.to_string())
                                .width(key_editor_width)
                                .show_ui(row_ui, |combo_ui| {
                                    use strum::IntoEnumIterator;
                                    for key_variant in CanonicalMetadataKey::iter() {
                                        if combo_ui
                                            .selectable_value(
                                                &mut entry.key,
                                                key_variant.clone(),
                                                key_variant.to_string(),
                                            )
                                            .changed()
                                        {
                                            key_changed_this_frame = true;
                                        }
                                    }
                                    combo_ui.separator();
                                    if combo_ui.selectable_label(false, "自定义").clicked() {
                                        entry.key =
                                            CanonicalMetadataKey::Custom("custom".to_string());
                                        key_changed_this_frame = true;
                                    }
                                });
                        }
                    } else {
                        let style = row_ui.style();
                        let space_for_pin_label = row_ui.text_style_height(&egui::TextStyle::Body);
                        let space_for_key_label =
                            style.spacing.item_spacing.x + style.spacing.interact_size.x;

                        row_ui.add_space(
                            space_for_pin_label + space_for_key_label + key_editor_width,
                        );
                    }

                    if key_changed_this_frame {
                        actions_to_send.push(UserAction::Lyrics(Box::new(
                            LyricsAction::UpdateMetadataKey(index, entry.key.clone()),
                        )));
                    }

                    row_ui.add_space(5.0);
                    row_ui.label("值:");
                    let value_edit_response = row_ui.add(
                        egui::TextEdit::singleline(&mut entry.value)
                            .id_salt(item_id.with("value_edit"))
                            .hint_text("元数据值"),
                    );
                    if value_edit_response.lost_focus() {
                        actions_to_send.push(UserAction::Lyrics(Box::new(
                            LyricsAction::UpdateMetadataValue(index, entry.value.clone()),
                        )));
                    }

                    if row_ui.button("🗑").on_hover_text("删除此条元数据").clicked() {
                        deletion_index = Some(index);
                    }
                });
                previous_key = Some(&entry.key);
            }

            if let Some(index_to_delete) = deletion_index {
                actions_to_send.push(UserAction::Lyrics(Box::new(LyricsAction::DeleteMetadata(
                    index_to_delete,
                ))));
            }

            scroll_ui.separator();

            scroll_ui.menu_button("添加新元数据...", |menu| {
                use strum::IntoEnumIterator;
                for key_variant in CanonicalMetadataKey::iter() {
                    if menu.button(key_variant.to_string()).clicked() {
                        actions_to_send.push(UserAction::Lyrics(Box::new(
                            LyricsAction::AddMetadata(key_variant),
                        )));
                        menu.close_menu();
                    }
                }
                menu.separator();
                if menu.button("自定义键").clicked() {
                    actions_to_send.push(UserAction::Lyrics(Box::new(LyricsAction::AddMetadata(
                        CanonicalMetadataKey::Custom("custom".to_string()),
                    ))));
                    menu.close_menu();
                }
            });
        });

        for action in actions_to_send {
            self.send_action(action);
        }
    }

    /// 绘制底部日志面板。
    pub fn draw_log_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("log_panel_id")
            .resizable(true)
            .default_height(150.0)
            .min_height(60.0)
            .max_height(ctx.available_rect().height() * 0.7)
            .show_animated(ctx, self.ui.show_bottom_log_panel, |ui| {
                ui.vertical_centered_justified(|ui_header| {
                    ui_header.horizontal(|h_ui| {
                        h_ui.label(egui::RichText::new("日志").strong());
                        h_ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |btn_ui| {
                                if btn_ui.button("关闭").clicked() {
                                    self.send_action(UserAction::UI(UIAction::HidePanel(
                                        crate::app_actions::PanelType::Log,
                                    )));
                                }
                                if btn_ui.button("清空").clicked() {
                                    self.send_action(UserAction::UI(UIAction::ClearLogs));
                                }
                            },
                        );
                    });
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .stick_to_bottom(true)
                    .show(ui, |scroll_ui| {
                        if self.ui.log_display_buffer.is_empty() {
                            scroll_ui.add_space(5.0);
                            scroll_ui.label(egui::RichText::new("暂无日志。").weak().italics());
                            scroll_ui.add_space(5.0);
                        } else {
                            for entry in &self.ui.log_display_buffer {
                                scroll_ui.horizontal_wrapped(|line_ui| {
                                    line_ui.label(
                                        egui::RichText::new(
                                            entry.timestamp.format("[%H:%M:%S.%3f]").to_string(),
                                        )
                                        .monospace(),
                                    );
                                    line_ui.add_space(4.0);
                                    line_ui.label(
                                        egui::RichText::new(format!("[{}]", entry.level.as_str()))
                                            .monospace()
                                            .color(entry.level.color())
                                            .strong(),
                                    );
                                    line_ui.add_space(4.0);
                                    line_ui.label(egui::RichText::new(&entry.message).monospace());
                                });
                            }
                        }
                        scroll_ui.allocate_space(scroll_ui.available_size_before_wrap());
                    });
            });
    }

    /// 绘制主歌词输入面板的内容。
    pub fn draw_input_panel_contents(&mut self, ui: &mut egui::Ui) {
        ui.add_space(TITLE_ALIGNMENT_OFFSET);
        ui.horizontal(|title_ui| {
            title_ui.heading("输入歌词");
            title_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |btn_ui| {
                if btn_ui
                    .add_enabled(
                        !self.lyrics.input_text.is_empty() || !self.lyrics.output_text.is_empty(),
                        egui::Button::new("清空"),
                    )
                    .clicked()
                {
                    self.send_action(crate::app_actions::UserAction::Lyrics(Box::new(
                        crate::app_actions::LyricsAction::ClearAllData,
                    )));
                }
                btn_ui.add_space(BUTTON_STRIP_SPACING);
                if btn_ui
                    .add_enabled(
                        !self.lyrics.input_text.is_empty(),
                        egui::Button::new("复制"),
                    )
                    .clicked()
                {
                    btn_ui.ctx().copy_text(self.lyrics.input_text.clone());
                }
                btn_ui.add_space(BUTTON_STRIP_SPACING);
                if btn_ui.button("粘贴").clicked() {
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        if let Ok(text) = clipboard.get_text() {
                            self.lyrics.input_text = text.clone();
                            self.send_action(crate::app_actions::UserAction::Lyrics(Box::new(
                                crate::app_actions::LyricsAction::MainInputChanged(text),
                            )));
                        } else {
                            tracing::error!("无法从剪贴板获取文本");
                        }
                    } else {
                        tracing::error!("无法访问剪贴板");
                    }
                }
            });
        });
        ui.separator();

        let scroll_area = if self.ui.wrap_text {
            egui::ScrollArea::vertical().id_salt("input_scroll_vertical_only")
        } else {
            egui::ScrollArea::both()
                .id_salt("input_scroll_both")
                .auto_shrink([false, false])
        };

        scroll_area.auto_shrink([false, false]).show(ui, |s_ui| {
            let text_edit_widget = egui::TextEdit::multiline(&mut self.lyrics.input_text)
                .hint_text("在此处粘贴或拖放主歌词文件")
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY);

            let response = if !self.ui.wrap_text {
                let font_id = egui::TextStyle::Monospace.resolve(s_ui.style());
                let text_color = s_ui.visuals().text_color();

                let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
                    let layout_job = egui::text::LayoutJob::simple(
                        string.to_string(),
                        font_id.clone(),
                        text_color,
                        f32::INFINITY,
                    );
                    ui.fonts(|f| f.layout_job(layout_job))
                };

                s_ui.add(text_edit_widget.layouter(&mut layouter))
            } else {
                s_ui.add(text_edit_widget)
            };

            if response.changed() && !self.lyrics.conversion_in_progress {
                self.send_action(crate::app_actions::UserAction::Lyrics(Box::new(
                    crate::app_actions::LyricsAction::MainInputChanged(
                        self.lyrics.input_text.clone(),
                    ),
                )));
            }
        });
    }

    /// 绘制翻译LRC面板的内容。
    pub fn draw_translation_lrc_panel_contents(&mut self, ui: &mut egui::Ui) {
        let mut text_edited_this_frame = false;

        let title = "翻译 (LRC)";
        let lrc_is_currently_considered_active = self.lyrics.loaded_translation_lrc.is_some()
            || !self.lyrics.display_translation_lrc_output.trim().is_empty();

        ui.add_space(TITLE_ALIGNMENT_OFFSET);
        ui.label(egui::RichText::new(title).heading());
        ui.separator();

        ui.horizontal(|button_strip_ui| {
            let main_lyrics_exist_for_merge = self.lyrics.parsed_lyric_data.as_ref().is_some();
            let import_enabled = main_lyrics_exist_for_merge && !self.lyrics.conversion_in_progress;
            let import_button_widget = egui::Button::new("导入");
            let mut import_button_response =
                button_strip_ui.add_enabled(import_enabled, import_button_widget);
            if !import_enabled {
                import_button_response =
                    import_button_response.on_disabled_hover_text("请先加载主歌词文件");
            }
            if import_button_response.clicked() {
                self.send_action(crate::app_actions::UserAction::File(
                    crate::app_actions::FileAction::LoadTranslationLrc,
                ));
            }

            button_strip_ui.allocate_ui_with_layout(
                button_strip_ui.available_size_before_wrap(),
                egui::Layout::right_to_left(egui::Align::Center),
                |right_aligned_buttons_ui| {
                    if right_aligned_buttons_ui
                        .add_enabled(
                            lrc_is_currently_considered_active,
                            egui::Button::new("清除"),
                        )
                        .clicked()
                    {
                        // 发送清除翻译LRC的事件
                        self.send_action(crate::app_actions::UserAction::Lyrics(Box::new(
                            crate::app_actions::LyricsAction::LrcInputChanged(
                                String::new(),
                                crate::types::LrcContentType::Translation,
                            ),
                        )));
                    }
                    right_aligned_buttons_ui.add_space(BUTTON_STRIP_SPACING);
                    if right_aligned_buttons_ui
                        .add_enabled(
                            !self.lyrics.display_translation_lrc_output.is_empty(),
                            egui::Button::new("复制"),
                        )
                        .clicked()
                    {
                        right_aligned_buttons_ui
                            .ctx()
                            .copy_text(self.lyrics.display_translation_lrc_output.clone());
                    }
                },
            );
        });

        let scroll_area = if self.ui.wrap_text {
            egui::ScrollArea::vertical().id_salt("translation_lrc_scroll_vertical")
        } else {
            egui::ScrollArea::both()
                .id_salt("translation_lrc_scroll_both")
                .auto_shrink([false, false])
        };

        scroll_area
            .auto_shrink([false, false])
            .show(ui, |s_ui_content| {
                let text_edit_widget =
                    egui::TextEdit::multiline(&mut self.lyrics.display_translation_lrc_output)
                        .hint_text("在此处粘贴翻译LRC内容")
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(10);

                let response = if !self.ui.wrap_text {
                    let font_id = egui::TextStyle::Monospace.resolve(s_ui_content.style());
                    let text_color = s_ui_content.visuals().text_color();

                    let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
                        let layout_job = egui::text::LayoutJob::simple(
                            string.to_string(),
                            font_id.clone(),
                            text_color,
                            f32::INFINITY,
                        );
                        ui.fonts(|f| f.layout_job(layout_job))
                    };
                    s_ui_content.add(text_edit_widget.layouter(&mut layouter))
                } else {
                    s_ui_content.add(text_edit_widget)
                };

                if response.changed() {
                    text_edited_this_frame = true;
                }
                s_ui_content.allocate_space(s_ui_content.available_size_before_wrap());
            });

        if text_edited_this_frame {
            // 只发送带有新文本内容的事件
            self.send_action(crate::app_actions::UserAction::Lyrics(Box::new(
                crate::app_actions::LyricsAction::LrcInputChanged(
                    self.lyrics.display_translation_lrc_output.clone(),
                    crate::types::LrcContentType::Translation,
                ),
            )));
        }
    }

    /// 绘制罗马音LRC面板的内容。
    pub fn draw_romanization_lrc_panel_contents(&mut self, ui: &mut egui::Ui) {
        let mut text_edited_this_frame = false;

        let title = "罗马音 (LRC)";
        let lrc_is_currently_considered_active = self.lyrics.loaded_romanization_lrc.is_some()
            || !self
                .lyrics
                .display_romanization_lrc_output
                .trim()
                .is_empty();

        ui.add_space(TITLE_ALIGNMENT_OFFSET);
        ui.label(egui::RichText::new(title).heading());
        ui.separator();

        ui.horizontal(|button_strip_ui| {
            let main_lyrics_exist_for_merge = self
                .lyrics
                .parsed_lyric_data
                .as_ref()
                .is_some_and(|p| !p.lines.is_empty());
            let import_enabled = main_lyrics_exist_for_merge && !self.lyrics.conversion_in_progress;
            let import_button_widget = egui::Button::new("导入");
            let mut import_button_response =
                button_strip_ui.add_enabled(import_enabled, import_button_widget);
            if !import_enabled {
                import_button_response =
                    import_button_response.on_disabled_hover_text("请先加载主歌词文件");
            }
            if import_button_response.clicked() {
                self.send_action(crate::app_actions::UserAction::File(
                    crate::app_actions::FileAction::LoadRomanizationLrc,
                ));
            }

            button_strip_ui.allocate_ui_with_layout(
                button_strip_ui.available_size_before_wrap(),
                egui::Layout::right_to_left(egui::Align::Center),
                |right_aligned_buttons_ui| {
                    if right_aligned_buttons_ui
                        .add_enabled(
                            lrc_is_currently_considered_active,
                            egui::Button::new("清除"),
                        )
                        .clicked()
                    {
                        // 发送清除罗马音LRC的事件
                        self.send_action(crate::app_actions::UserAction::Lyrics(Box::new(
                            crate::app_actions::LyricsAction::LrcInputChanged(
                                String::new(),
                                crate::types::LrcContentType::Romanization,
                            ),
                        )));
                    }
                    right_aligned_buttons_ui.add_space(BUTTON_STRIP_SPACING);
                    if right_aligned_buttons_ui
                        .add_enabled(
                            !self.lyrics.display_romanization_lrc_output.is_empty(),
                            egui::Button::new("复制"),
                        )
                        .clicked()
                    {
                        right_aligned_buttons_ui
                            .ctx()
                            .copy_text(self.lyrics.display_romanization_lrc_output.clone());
                    }
                },
            );
        });

        let scroll_area = if self.ui.wrap_text {
            egui::ScrollArea::vertical().id_salt("romanization_lrc_scroll_vertical")
        } else {
            egui::ScrollArea::both()
                .id_salt("romanization_lrc_scroll_both")
                .auto_shrink([false, false])
        };

        scroll_area
            .auto_shrink([false, false])
            .show(ui, |s_ui_content| {
                let text_edit_widget =
                    egui::TextEdit::multiline(&mut self.lyrics.display_romanization_lrc_output)
                        .hint_text("在此处粘贴罗马音LRC内容")
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(10);

                let response = if !self.ui.wrap_text {
                    let font_id = egui::TextStyle::Monospace.resolve(s_ui_content.style());
                    let text_color = s_ui_content.visuals().text_color();

                    let mut layouter = |ui: &egui::Ui, string: &str, _wrap_width: f32| {
                        let layout_job = egui::text::LayoutJob::simple(
                            string.to_string(),
                            font_id.clone(),
                            text_color,
                            f32::INFINITY,
                        );
                        ui.fonts(|f| f.layout_job(layout_job))
                    };
                    s_ui_content.add(text_edit_widget.layouter(&mut layouter))
                } else {
                    s_ui_content.add(text_edit_widget)
                };

                if response.changed() {
                    text_edited_this_frame = true;
                }
                s_ui_content.allocate_space(s_ui_content.available_size_before_wrap());
            });

        if text_edited_this_frame {
            // 只发送带有新文本内容的事件
            self.send_action(crate::app_actions::UserAction::Lyrics(Box::new(
                crate::app_actions::LyricsAction::LrcInputChanged(
                    self.lyrics.display_romanization_lrc_output.clone(),
                    crate::types::LrcContentType::Romanization,
                ),
            )));
        }
    }

    /// 绘制输出结果面板的内容。
    pub fn draw_output_panel_contents(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|title_ui| {
            title_ui.heading("输出结果");
            title_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |btn_ui| {
                let send_to_player_enabled;
                {
                    let connector_config_guard = self.amll_connector.config.lock().unwrap();
                    send_to_player_enabled = connector_config_guard.enabled
                        && self.lyrics.parsed_lyric_data.is_some()
                        && !self.lyrics.conversion_in_progress;
                }

                let send_button = Button::new("发送到AMLL Player");
                let mut send_button_response =
                    btn_ui.add_enabled(send_to_player_enabled, send_button);

                if !send_to_player_enabled {
                    send_button_response = send_button_response
                        .on_disabled_hover_text("需要先成功转换出可用的歌词数据");
                }

                if send_button_response.clicked()
                    && let (Some(tx), Some(parsed_data)) = (
                        &self.amll_connector.command_tx,
                        self.lyrics.parsed_lyric_data.as_ref(),
                    )
                {
                    if tx
                        .try_send(crate::amll_connector::ConnectorCommand::SendLyric(
                            parsed_data.clone(),
                        ))
                        .is_err()
                    {
                        tracing::error!("[Unilyric UI] 手动发送歌词失败。");
                    } else {
                        tracing::info!("[Unilyrc UI] 已从输出面板手动发送歌词。");
                    }
                }

                btn_ui.add_space(BUTTON_STRIP_SPACING);

                if btn_ui
                    .add_enabled(
                        !self.lyrics.output_text.is_empty() && !self.lyrics.conversion_in_progress,
                        Button::new("复制"),
                    )
                    .clicked()
                {
                    btn_ui.ctx().copy_text(self.lyrics.output_text.clone());
                    self.ui.toasts.add(egui_toast::Toast {
                        text: "输出内容已复制到剪贴板".into(),
                        kind: egui_toast::ToastKind::Success,
                        options: egui_toast::ToastOptions::default().duration_in_seconds(2.0),
                        style: Default::default(),
                    });
                }
            });
        });
        ui.separator();

        let scroll_area = if self.ui.wrap_text {
            ScrollArea::vertical().id_salt("output_scroll_vertical_label")
        } else {
            ScrollArea::both()
                .id_salt("output_scroll_both_label")
                .auto_shrink([false, false])
        };

        scroll_area.auto_shrink([false, false]).show(ui, |s_ui| {
            let mut label_widget = egui::Label::new(
                egui::RichText::new(&self.lyrics.output_text)
                    .monospace()
                    .size(13.0),
            )
            .selectable(true);

            if self.ui.wrap_text {
                label_widget = label_widget.wrap();
            } else {
                label_widget = label_widget.extend();
            }
            s_ui.add(label_widget);
        });
    }

    pub fn draw_amll_connector_sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(TITLE_ALIGNMENT_OFFSET);
        ui.heading("AMLL Connector");
        ui.separator();

        ui.strong("AMLL Player 连接:");

        ui.vertical(|ui| {
            let current_status = self.amll_connector.status.lock().unwrap().clone();
            let websocket_url_display = self
                .amll_connector
                .config
                .lock()
                .unwrap()
                .websocket_url
                .clone();

            ui.label(format!("目标 URL: {websocket_url_display}"));

            match current_status {
                WebsocketStatus::Disconnected => {
                    if ui.button("连接到 AMLL Player").clicked() {
                        self.send_action(UserAction::AmllConnector(AmllConnectorAction::Connect));
                    }
                    ui.weak("状态: 未连接");
                }
                WebsocketStatus::Connecting => {
                    ui.horizontal(|h_ui| {
                        h_ui.add(Spinner::new());
                        h_ui.label("正在连接...");
                    });
                }
                WebsocketStatus::Connected => {
                    if ui.button("断开连接").clicked() {
                        self.send_action(UserAction::AmllConnector(
                            AmllConnectorAction::Disconnect,
                        ));
                    }
                    ui.colored_label(Color32::GREEN, "状态: 已连接");
                }
                WebsocketStatus::Error(err_msg_ref) => {
                    if ui.button("重试连接").clicked() {
                        self.send_action(UserAction::AmllConnector(AmllConnectorAction::Retry));
                    }
                    ui.colored_label(Color32::RED, "状态: 错误");
                    ui.small(err_msg_ref);
                }
            }
        });

        ui.separator();

        ui.strong("SMTC 源应用:");

        let available_sessions = self.player.available_sessions.clone();
        let mut selected_id = self.player.last_requested_session_id.clone();

        let combo_label_text = match selected_id.as_ref() {
            Some(id) => available_sessions
                .iter()
                .find(|s| &s.session_id == id)
                .map_or_else(
                    || format!("自动 (之前选择的 '{id}' 已失效)"),
                    |s_info| s_info.display_name.clone(),
                ),
            None => "自动 (系统默认)".to_string(),
        };

        let combo_changed = egui::ComboBox::from_id_salt("smtc_source_selector")
            .selected_text(combo_label_text)
            .show_ui(ui, |combo_ui| {
                let mut changed_in_combo = false;
                if combo_ui
                    .selectable_label(selected_id.is_none(), "自动 (系统默认)")
                    .clicked()
                {
                    selected_id = None;
                    changed_in_combo = true;
                }
                for session_info in &available_sessions {
                    if combo_ui
                        .selectable_label(
                            selected_id.as_ref() == Some(&session_info.session_id),
                            &session_info.display_name,
                        )
                        .clicked()
                    {
                        selected_id = Some(session_info.session_id.clone());
                        changed_in_combo = true;
                    }
                }
                changed_in_combo
            })
            .inner
            .unwrap_or(false);

        if combo_changed {
            self.send_action(UserAction::Player(PlayerAction::SelectSmtcSession(
                selected_id.unwrap_or_default(),
            )));
        }

        ui.separator();
        ui.strong("当前监听 (SMTC):");

        let now_playing = &self.player.current_now_playing;
        if now_playing.title.is_some() {
            ui.label(format!(
                "歌曲: {}",
                now_playing.title.as_deref().unwrap_or("未知")
            ));
            ui.label(format!(
                "艺术家: {}",
                now_playing.artist.as_deref().unwrap_or("未知")
            ));
            ui.label(format!(
                "专辑: {}",
                now_playing.album_title.as_deref().unwrap_or("未知")
            ));

            if let Some(status) = now_playing.playback_status {
                ui.label(match status {
                    smtc_suite::PlaybackStatus::Playing => "状态: 播放中",
                    smtc_suite::PlaybackStatus::Paused => "状态: 已暂停",
                    smtc_suite::PlaybackStatus::Stopped => "状态: 已停止",
                });
            }

            if let Some(cover_bytes) = &now_playing.cover_data
                && !cover_bytes.is_empty()
            {
                let image_id_cow = now_playing.cover_data_hash.map_or_else(
                    || "smtc_cover_no_hash".into(),
                    |hash| format!("smtc_cover_hash_{hash}").into(),
                );
                let image_source = egui::ImageSource::Bytes {
                    uri: image_id_cow,
                    bytes: cover_bytes.clone().into(),
                };
                ui.add_sized(
                    egui::vec2(200.0, 200.0),
                    egui::Image::new(image_source)
                        .max_size(egui::vec2(200.0, 200.0))
                        .maintain_aspect_ratio(true)
                        .bg_fill(Color32::TRANSPARENT),
                );
            }

            ui.strong("时间轴偏移:");
            let mut offset_action_to_send = None;
            ui.horizontal(|h_ui| {
                h_ui.label("偏移量:");
                let mut current_offset = self.player.smtc_time_offset_ms;
                let response = h_ui.add(
                    egui::DragValue::new(&mut current_offset)
                        .speed(10.0)
                        .suffix(" ms"),
                );
                if response.changed() {
                    offset_action_to_send = Some(UserAction::Player(
                        PlayerAction::SetSmtcTimeOffset(current_offset),
                    ));
                }
            });

            if let Some(action) = offset_action_to_send {
                self.send_action(action);
            }
        } else {
            ui.weak("无SMTC信息 / 未选择特定源");
        }

        ui.separator();

        ui.strong("本地歌词:");
        let can_save_to_local =
            !self.lyrics.output_text.is_empty() && self.player.current_now_playing.title.is_some();

        let save_button_widget = Button::new("💾 保存输出框歌词到本地");
        let mut response = ui.add_enabled(can_save_to_local, save_button_widget);
        if !can_save_to_local {
            response = response.on_disabled_hover_text("需先有歌词输出和媒体信息才能缓存");
        }
        if response.clicked() {
            self.send_action(UserAction::Player(PlayerAction::SaveToLocalCache));
        }

        ui.separator();

        ui.strong("自动歌词搜索状态:");
        let sources_config = vec![
            (
                AutoSearchSource::LocalCache,
                &self.fetcher.local_cache_status,
                None,
            ),
            (
                AutoSearchSource::QqMusic,
                &self.fetcher.qqmusic_status,
                Some(&self.fetcher.last_qq_result),
            ),
            (
                AutoSearchSource::Kugou,
                &self.fetcher.kugou_status,
                Some(&self.fetcher.last_kugou_result),
            ),
            (
                AutoSearchSource::Netease,
                &self.fetcher.netease_status,
                Some(&self.fetcher.last_netease_result),
            ),
            (
                AutoSearchSource::AmllDb,
                &self.fetcher.amll_db_status,
                Some(&self.fetcher.last_amll_db_result),
            ),
        ];

        let mut action_load_lyrics: Option<(AutoSearchSource, FullLyricsResult)> = None;
        let mut action_refetch: Option<AutoSearchSource> = None;

        for (source_enum, status_arc, opt_result_arc) in sources_config {
            ui.horizontal(|item_ui| {
                item_ui.label(format!("{}:", source_enum.display_name()));
                let status = status_arc.lock().unwrap().clone();

                item_ui.with_layout(Layout::right_to_left(Align::Center), |right_aligned_ui| {
                    let mut stored_data_for_load: Option<FullLyricsResult> = None;
                    if let Some(result_arc) = opt_result_arc
                        && let Some(ref data) = *result_arc.lock().unwrap()
                    {
                        stored_data_for_load = Some(data.clone());
                    }

                    if let Some(data) = stored_data_for_load {
                        if right_aligned_ui
                            .button("载入")
                            .on_hover_text(format!(
                                "使用 {} 找到的歌词",
                                source_enum.display_name()
                            ))
                            .clicked()
                        {
                            action_load_lyrics = Some((source_enum, data));
                        }
                        right_aligned_ui.add_space(4.0);
                    }

                    if source_enum != AutoSearchSource::LocalCache
                        && right_aligned_ui.button("重搜").clicked()
                    {
                        action_refetch = Some(source_enum);
                    }

                    let status_display_text = match status {
                        AutoSearchStatus::NotAttempted => "未尝试".to_string(),
                        AutoSearchStatus::Searching => "正在搜索...".to_string(),
                        AutoSearchStatus::Success(_) => "已找到".to_string(),
                        AutoSearchStatus::NotFound => "未找到".to_string(),
                        AutoSearchStatus::Error(_) => "错误".to_string(),
                    };

                    if let AutoSearchStatus::Searching = status {
                        right_aligned_ui.spinner();
                    }
                    right_aligned_ui.label(status_display_text);
                });
            });
        }

        if let Some((_source, result)) = action_load_lyrics {
            self.send_action(UserAction::Lyrics(Box::new(
                LyricsAction::LoadFetchedResult(result),
            )));
        }
        if let Some(source) = action_refetch {
            crate::app_fetch_core::trigger_manual_refetch_for_source(self, source);
        }
    }

    /// 绘制歌词搜索/下载窗口。
    pub fn draw_downloader_view(&mut self, ctx: &egui::Context) {
        if matches!(
            self.lyrics_helper_state.provider_state,
            crate::types::ProviderState::Uninitialized
        ) {
            self.trigger_provider_loading();
        }

        let mut action_to_send = None;

        egui::SidePanel::left("downloader_left_panel")
            .resizable(true)
            .default_width(300.0)
            .width_range(250.0..=500.0)
            .show(ctx, |left_ui| {
                left_ui.horizontal(|header_ui| {
                    header_ui.heading("搜索");
                    header_ui.with_layout(Layout::right_to_left(Align::Center), |btn_ui| {
                        if btn_ui.button("返回").clicked() {
                            action_to_send =
                                Some(UserAction::Downloader(Box::new(DownloaderAction::Close)));
                        }
                    });
                });

                left_ui.separator();
                let is_searching = matches!(self.downloader.search_state, SearchState::Searching);

                let mut perform_search = false;

                egui::Grid::new("search_inputs_grid")
                    .num_columns(2)
                    .show(left_ui, |grid_ui| {
                        grid_ui.label("歌曲名:");
                        let title_edit = grid_ui.add_enabled(
                            !is_searching,
                            TextEdit::singleline(&mut self.downloader.title_input)
                                .hint_text("必填"),
                        );
                        if title_edit.lost_focus()
                            && grid_ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            perform_search = true;
                        }
                        grid_ui.end_row();

                        grid_ui.label("艺术家:");
                        let artist_edit = grid_ui.add_enabled(
                            !is_searching,
                            TextEdit::singleline(&mut self.downloader.artist_input)
                                .hint_text("可选"),
                        );
                        if artist_edit.lost_focus()
                            && grid_ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            perform_search = true;
                        }
                        grid_ui.end_row();

                        grid_ui.label("专辑:");
                        let album_edit = grid_ui.add_enabled(
                            !is_searching,
                            TextEdit::singleline(&mut self.downloader.album_input)
                                .hint_text("可选"),
                        );
                        if album_edit.lost_focus()
                            && grid_ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            perform_search = true;
                        }
                        grid_ui.end_row();

                        grid_ui.label("时长 (ms):");
                        grid_ui.add_enabled(
                            !is_searching,
                            egui::DragValue::new(&mut self.downloader.duration_ms_input)
                                .speed(1000.0),
                        );
                        grid_ui.end_row();
                    });

                left_ui.horizontal(|h_ui| {
                    let providers_ready = matches!(
                        self.lyrics_helper_state.provider_state,
                        crate::types::ProviderState::Ready
                    );
                    let search_enabled =
                        !self.downloader.title_input.is_empty() && !is_searching && providers_ready;

                    if h_ui
                        .add_enabled(search_enabled, Button::new("搜索"))
                        .clicked()
                    {
                        perform_search = true;
                    }

                    if h_ui.button("从SMTC填充").clicked() {
                        action_to_send = Some(UserAction::Downloader(Box::new(
                            DownloaderAction::FillFromSmtc,
                        )));
                    }

                    if is_searching {
                        h_ui.add(Spinner::new());
                    }
                });

                if perform_search {
                    action_to_send = Some(UserAction::Downloader(Box::new(
                        DownloaderAction::PerformSearch,
                    )));
                }

                left_ui.add_space(10.0);
                left_ui.heading("搜索结果");
                left_ui.separator();

                ScrollArea::vertical().auto_shrink([false, false]).show(
                    left_ui,
                    |s_ui| match &self.downloader.search_state {
                        SearchState::Idle => {
                            s_ui.label("请输入关键词进行搜索。");
                        }
                        SearchState::Searching => {
                            s_ui.label("正在搜索...");
                        }
                        SearchState::Error(err) => {
                            s_ui.colored_label(Color32::RED, "搜索失败:");
                            s_ui.label(err);
                        }
                        SearchState::Success(results) => {
                            if results.is_empty() {
                                s_ui.label("未找到结果。");
                            } else {
                                for result in results {
                                    let is_selected =
                                        self.downloader.selected_result_for_preview.as_ref()
                                            == Some(result);

                                    let artists_str = result
                                        .artists
                                        .iter()
                                        .map(|a| a.name.as_str())
                                        .collect::<Vec<_>>()
                                        .join("/");

                                    let album_str = result.album.as_deref().unwrap_or("未知专辑");

                                    let duration_str = result.duration.map_or_else(
                                        || "未知时长".to_string(),
                                        |ms| {
                                            let secs = ms / 1000;
                                            format!("{:02}:{:02}", secs / 60, secs % 60)
                                        },
                                    );

                                    let display_text = format!(
                                        "{} - {}\n专辑: {}\n时长: {} | 来源: {} | 匹配度: {:?}",
                                        result.title,
                                        artists_str,
                                        album_str,
                                        duration_str,
                                        result.provider_name,
                                        result.match_type
                                    );
                                    if s_ui.selectable_label(is_selected, display_text).clicked() {
                                        action_to_send = Some(UserAction::Downloader(Box::new(
                                            DownloaderAction::SelectResultForPreview(
                                                result.clone(),
                                            ),
                                        )));
                                    }
                                }
                            }
                        }
                    },
                );
            });

        egui::CentralPanel::default().show(ctx, |right_ui| {
            right_ui.heading("歌词预览");
            right_ui.separator();

            match &self.downloader.preview_state {
                PreviewState::Idle => {}
                PreviewState::Loading => {
                    right_ui.centered_and_justified(|cj_ui| {
                        cj_ui.vertical_centered(|vc_ui| {
                            vc_ui.add(Spinner::new());
                        });
                    });
                }
                PreviewState::Error(err) => {
                    right_ui.centered_and_justified(|cj_ui| {
                        cj_ui.label(format!("预览加载失败:\n{}", err));
                    });
                }
                PreviewState::Success(preview_text) => {
                    let can_apply = self.downloader.selected_full_lyrics.is_some();
                    egui::TopBottomPanel::bottom("preview_actions_panel").show_inside(
                        right_ui,
                        |bottom_ui| {
                            bottom_ui.with_layout(Layout::right_to_left(Align::Center), |btn_ui| {
                                if btn_ui.add_enabled(can_apply, Button::new("应用")).clicked() {
                                    action_to_send = Some(UserAction::Downloader(Box::new(
                                        DownloaderAction::ApplyAndClose,
                                    )));
                                }
                            });
                        },
                    );

                    egui::CentralPanel::default().show_inside(right_ui, |text_panel_ui| {
                        ScrollArea::vertical().auto_shrink([false, false]).show(
                            text_panel_ui,
                            |s_ui| {
                                s_ui.add(
                                    egui::Label::new(egui::RichText::new(preview_text).monospace())
                                        .selectable(true)
                                        .wrap(),
                                );
                            },
                        );
                    });
                }
            }
        });

        if let Some(action) = action_to_send {
            self.send_action(action);
        }
    }

    pub fn draw_batch_converter_view(&mut self, ctx: &egui::Context) {
        use crate::app_actions::BatchConverterAction;

        egui::TopBottomPanel::top("batch_converter_toolbar").show(ctx, |ui| {
            egui::menu::bar(ui, |bar_ui| {
                if bar_ui.button("返回").clicked() {
                    self.send_action(UserAction::UI(UIAction::SetView(AppView::Editor)));
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("批量歌词转换器");
            ui.separator();

            ui.horizontal(|h_ui| {
                h_ui.strong("输入目录:");
                if let Some(path) = &self.batch_converter.input_dir {
                    h_ui.monospace(path.to_string_lossy());
                } else {
                    h_ui.weak("未选择");
                }
                if h_ui.button("选择...").clicked() {
                    self.send_action(UserAction::BatchConverter(
                        BatchConverterAction::SelectInputDir,
                    ));
                }
            });

            ui.horizontal(|h_ui| {
                h_ui.strong("输出目录:");
                if let Some(path) = &self.batch_converter.output_dir {
                    h_ui.monospace(path.to_string_lossy());
                } else {
                    h_ui.weak("未选择");
                }
                if h_ui.button("选择...").clicked() {
                    self.send_action(UserAction::BatchConverter(
                        BatchConverterAction::SelectOutputDir,
                    ));
                }
            });

            ui.add_space(10.0);

            let can_scan = self.batch_converter.input_dir.is_some()
                && self.batch_converter.output_dir.is_some()
                && self.batch_converter.status != BatchConverterStatus::Converting;

            let scan_button = ui.add_enabled(can_scan, egui::Button::new("扫描任务"));
            if scan_button.clicked() {
                self.send_action(UserAction::BatchConverter(BatchConverterAction::ScanTasks));
            }
            if !can_scan && self.batch_converter.input_dir.is_none() {
                scan_button.on_disabled_hover_text("请先选择输入目录");
            } else if !can_scan && self.batch_converter.output_dir.is_none() {
                scan_button.on_disabled_hover_text("请先选择输出目录");
            }

            ui.separator();

            ui.heading("转换任务");

            let status_text = match self.batch_converter.status {
                BatchConverterStatus::Idle => "等待扫描...".to_string(),
                BatchConverterStatus::Ready => format!(
                    "已扫描 {} 个任务, 等待开始。",
                    self.batch_converter.tasks.len()
                ),
                BatchConverterStatus::Converting => "正在转换...".to_string(),
                BatchConverterStatus::Completed => {
                    format!("已完成所有 {} 个任务。", self.batch_converter.tasks.len())
                }
                BatchConverterStatus::Failed(ref err) => format!("失败: {}", err),
            };
            ui.label(status_text);

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |scroll_ui| {
                    egui::Grid::new("batch_tasks_grid")
                        .num_columns(3)
                        .striped(true)
                        .show(scroll_ui, |grid_ui| {
                            grid_ui.strong("主文件");
                            grid_ui.strong("状态");
                            grid_ui.strong("详情");
                            grid_ui.end_row();

                            for task in &self.batch_converter.tasks {
                                if let Some(main_file) =
                                    self.batch_converter.file_lookup.get(&task.main_lyric_id)
                                {
                                    grid_ui.label(&main_file.filename);
                                } else {
                                    grid_ui.label("未知文件");
                                }

                                match &task.status {
                                    lyrics_helper_core::BatchEntryStatus::Pending => {
                                        grid_ui.label("等待中");
                                    }
                                    lyrics_helper_core::BatchEntryStatus::ReadyToConvert => {
                                        grid_ui.label("准备就绪");
                                    }
                                    lyrics_helper_core::BatchEntryStatus::Converting => {
                                        grid_ui.horizontal(|h| {
                                            h.add(egui::Spinner::new());
                                            h.label("转换中...");
                                        });
                                    }
                                    lyrics_helper_core::BatchEntryStatus::Completed { .. } => {
                                        grid_ui.colored_label(egui::Color32::GREEN, "完成");
                                    }
                                    lyrics_helper_core::BatchEntryStatus::Failed(_) => {
                                        grid_ui.colored_label(egui::Color32::RED, "失败");
                                    }
                                    lyrics_helper_core::BatchEntryStatus::SkippedNoMatch => {
                                        grid_ui.label("已跳过");
                                    }
                                };

                                if let lyrics_helper_core::BatchEntryStatus::Failed(err_msg) =
                                    &task.status
                                {
                                    grid_ui.label(err_msg);
                                } else if let lyrics_helper_core::BatchEntryStatus::Completed {
                                    output_path,
                                    ..
                                } = &task.status
                                {
                                    grid_ui.label(output_path.to_string_lossy());
                                } else {
                                    grid_ui.label("");
                                }
                                grid_ui.end_row();
                            }
                        });
                });

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |bottom_ui| {
                bottom_ui.add_space(10.0);
                bottom_ui.horizontal(|h_ui| {
                    let can_start_conversion =
                        self.batch_converter.status == BatchConverterStatus::Ready;
                    if h_ui
                        .add_enabled(can_start_conversion, egui::Button::new("开始转换"))
                        .clicked()
                    {
                        self.send_action(UserAction::BatchConverter(
                            BatchConverterAction::StartConversion,
                        ));
                    }
                    if h_ui.button("重置").clicked() {
                        self.send_action(UserAction::BatchConverter(BatchConverterAction::Reset));
                    }
                });
            });
        });
    }

    pub fn draw_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("app_status_bar").show(ctx, |ui| {
            ui.horizontal_centered(|h_ui| {
                h_ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |right_ui| {
                        let warnings_count = self.lyrics.current_warnings.len();
                        if warnings_count > 0 {
                            let button_text = format!("⚠️ {}", warnings_count);
                            let button = right_ui.button(button_text);
                            if button.clicked() {
                                self.send_action(UserAction::UI(UIAction::ShowPanel(
                                    PanelType::Warnings,
                                )));
                            }
                        }
                    },
                );
            });
        });
    }

    pub fn draw_warnings_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("warnings_panel_id")
            .resizable(true)
            .default_height(150.0)
            .min_height(60.0)
            .show_animated(ctx, self.ui.show_warnings_panel, |ui| {
                ui.vertical_centered_justified(|ui_header| {
                    ui_header.horizontal(|h_ui| {
                        h_ui.label(egui::RichText::new("解析警告").strong());
                        h_ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |btn_ui| {
                                if btn_ui.button("关闭").clicked() {
                                    self.send_action(UserAction::UI(UIAction::HidePanel(
                                        PanelType::Warnings,
                                    )));
                                }
                            },
                        );
                    });
                });
                ui.separator();

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |scroll_ui| {
                        if self.lyrics.current_warnings.is_empty() {
                            scroll_ui.label(egui::RichText::new("暂无警告。").weak().italics());
                        } else {
                            for warning in &self.lyrics.current_warnings {
                                scroll_ui.horizontal_wrapped(|line_ui| {
                                    line_ui.label("⚠️");
                                    line_ui.label(warning);
                                });
                            }
                        }
                    });
            });
    }
}
