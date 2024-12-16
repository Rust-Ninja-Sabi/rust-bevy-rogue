use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use bevy_egui::egui::Color32;
use crate::fighting::Actor;
use crate::{GameMap, Inventory, Player, ShowFps};

#[derive(Component)]
pub struct HeadUpDisplay{
    width:usize,
    height:usize,
    text:String
}

impl HeadUpDisplay{
    pub fn new()->Self{
        let width = 22;
        let height = 8;

        let player_position = (0,0);
        let text = String::from("");

        Self{
            width,
            height,
            text
        }
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update,update_headupdisplay);
        app.add_systems(Update,render_ui);
    }
}

fn update_headupdisplay(
    game_map: Res<GameMap>,
    mut query_player:Query<(&mut HeadUpDisplay, &Transform),Changed<Transform>>
){
    for (mut display,player_transform) in query_player.iter_mut() {
        let player_position = game_map.world_to_grid(player_transform.translation);
        let middle_position = (display.width / 2-1, display.height / 2-1);
        let display_position = (
            (player_position.0 as i32 - middle_position.0 as i32).max(middle_position.0 as i32),
            (player_position.1 as i32 - middle_position.1 as i32).max(middle_position.1 as i32)
        );

        display.text = game_map.to_string(
            display_position,
            player_position,
            display.width,display.height
        );
    }
}


fn render_ui(
    mut egui_context: EguiContexts,
    query_display: Query<&HeadUpDisplay>,
    query: Query<&Actor, With<Player>>,
    show_fps: ResMut<ShowFps>,
    diagnostics: Res<DiagnosticsStore>,
    inventory: Res<Inventory>,
) {
    if let Ok(actor) = query.get_single() {
        let neon_green = egui::Color32::from_rgb(57, 255, 20);

        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: egui::Color32::TRANSPARENT,
                ..Default::default()
            })
            .show(egui_context.ctx_mut(), |ui| {
                let panel_rect = ui.available_rect_before_wrap();

                if show_fps.0 {
                    if let Some(value) = diagnostics
                        .get(&FrameTimeDiagnosticsPlugin::FPS)
                        .and_then(|fps| fps.smoothed())
                    {
                        // Neues FPS-Feld links oben
                        let fps_rect = egui::Rect {
                            min: panel_rect.left_top() + egui::vec2(10.0, 10.0),
                            max: panel_rect.left_top() + egui::vec2(120.0, 50.0),
                        };

                        ui.allocate_ui_at_rect(fps_rect, |ui| {
                            egui::Frame {
                                fill: egui::Color32::TRANSPARENT,
                                stroke: egui::Stroke {
                                    width: 2.0,
                                    color: neon_green,
                                },
                                ..Default::default()
                            }
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new(format!("FPS: {:.1}", value))
                                            .color(neon_green)
                                            .monospace(),
                                    );
                                });
                        });
                    }
                }
                // Variabler Text rechts oben
                if let Some(display) = query_display.iter().next() {
                    let text_size = ui.fonts(|fonts| {
                        fonts.glyph_width(&egui::TextStyle::Monospace.resolve(&ui.style()), ' ') * 20.0
                    });
                    let text_height = ui.text_style_height(&egui::TextStyle::Monospace) * 6.0;

                    let text_rect = egui::Rect {
                        min: panel_rect.right_top() - egui::vec2(text_size + 20.0, -10.0),
                        max: panel_rect.right_top() + egui::vec2(0.0, text_height + 10.0),
                    };

                    ui.allocate_ui_at_rect(text_rect, |ui| {
                        egui::Frame {
                            fill: egui::Color32::TRANSPARENT,
                            stroke: egui::Stroke {
                                width: 2.0,
                                color: neon_green,
                            },
                            ..Default::default()
                        }
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(&display.text)
                                        .color(neon_green)
                                        .monospace(),
                                );
                            });
                    });
                }

                // Fortschrittsanzeige
                let progress_width = 600.0; // Increased width to accommodate labels
                let progress_height = 80.0;

                let progress_rect = egui::Rect {
                    min: panel_rect.center_bottom() - egui::vec2(progress_width / 2.0, progress_height / 2.0),
                    max: panel_rect.center_bottom() + egui::vec2(progress_width / 2.0, progress_height / 2.0),
                };

                ui.allocate_ui_at_rect(progress_rect, |ui| {
                    ui.horizontal(|ui| {
                        // Inventory Label with Frame
                        egui::Frame {
                            fill: egui::Color32::TRANSPARENT,
                            stroke: egui::Stroke {
                                width: 1.0,
                                color: neon_green,
                            },
                            inner_margin: egui::Margin::same(5.0),
                            ..Default::default()
                        }.show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("<I>nventory")
                                    .color(neon_green)
                                    .monospace()
                            );
                        });

                        // Active Item
                        egui::Frame {
                            fill: egui::Color32::TRANSPARENT,
                            stroke: egui::Stroke {
                                width: 1.0,
                                color: neon_green,
                            },
                            inner_margin: egui::Margin::same(5.0),
                            ..Default::default()
                        }.show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(inventory.get_active_item_name())
                                    .color(neon_green)
                                    .monospace()
                            );
                        });

                        // Progress Bar with Frame
                        egui::Frame {
                            fill: egui::Color32::TRANSPARENT,
                            stroke: egui::Stroke {
                                width: 1.0,
                                color: neon_green,
                            },
                            inner_margin: egui::Margin::same(5.0),
                            ..Default::default()
                        }.show(ui, |ui| {
                            ui.add(
                                egui::ProgressBar::new(actor.hit_points as f32 / actor.max_hit_points as f32)
                                    .text(format!("Health {}({})", actor.hit_points, actor.max_hit_points))
                            );
                        });

                        // Portion Label with Frame
                        egui::Frame {
                            fill: egui::Color32::TRANSPARENT,
                            stroke: egui::Stroke {
                                width: 1.0,
                                color: neon_green,
                            },
                            inner_margin: egui::Margin::same(5.0),
                            ..Default::default()
                        }.show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(format!("<P>ortion {}",inventory.heal_potion))
                                    .color(neon_green)
                                    .monospace()
                            );
                        });
                    });
                });
            });
    }
}