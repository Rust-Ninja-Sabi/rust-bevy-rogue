use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use bevy_egui::egui::Color32;
use crate::fighting::Actor;
use crate::{GameMap, Player};

#[derive(Component)]
pub struct HeadUpDisplay{
    width:usize,
    height:usize,
    text:String,
    player_position: (usize,usize)
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
            text,
            player_position
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
        let display_position = ( (player_position.0 - middle_position.0).max(middle_position.0) as usize ,
                                 (player_position.1 - middle_position.1).max(middle_position.1) as usize );
        display.player_position = (player_position.0 - display_position.0,
                                   player_position.1 - display_position.1);
        display.text = game_map.to_string(display_position,player_position,display.width,display.height);
    }
}

fn render_ui(
    mut egui_context: EguiContexts,
    query_display: Query<&HeadUpDisplay>,
    query: Query<&Actor, With<Player>>,
) {
    if let Ok(actor) = query.get_single() {
        let text = "....................\n\
                    .........####.......\n\
                    ....................\n\
                    ..........@.........\n\
                    ....................\n\
                    ....................";

        let neon_green = egui::Color32::from_rgb(57, 255, 20);

        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: egui::Color32::TRANSPARENT,
                ..Default::default()
            })
            .show(egui_context.ctx_mut(), |ui| {
                let panel_rect = ui.available_rect_before_wrap();

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
                let progress_width = 300.0;
                let progress_height = 40.0;

                let progress_rect = egui::Rect {
                    min: panel_rect.center_bottom() - egui::vec2(progress_width / 2.0, progress_height / 2.0),
                    max: panel_rect.center_bottom() + egui::vec2(progress_width / 2.0, progress_height / 2.0),
                };

                ui.allocate_ui_at_rect(progress_rect, |ui| {
                    ui.add(
                        egui::ProgressBar::new(actor.hit_points as f32 / actor.max_hit_points as f32)
                            .text(format!("Health {}({})", actor.hit_points, actor.max_hit_points)),
                    );
                });
            });
    }
}


fn render_uii(
    mut egui_context: EguiContexts,
    query: Query<&Actor, With<Player>>
) {

    if let Ok(actor) = query.get_single() {
        // do something with the components
        let my_frame = egui::containers::Frame {
            fill: Color32::from_rgba_premultiplied(0, 0, 0, 0),
            ..Default::default()
        };
        let text = "....................\n\
                .........####.......\n\
                ....................\n\
                ..........@.........\n\
                ....................\n\
                ....................";

        // Definiere die Neongrün-Farbe
        let neon_green = egui::Color32::from_rgb(57, 255, 20);

        egui::TopBottomPanel::top("text_panel").show(egui_context.ctx_mut(), |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                // Stil des Rahmens definieren
                let frame = egui::Frame {
                    fill: egui::Color32::TRANSPARENT,
                    stroke: egui::Stroke {
                        width: 2.0,            // Rahmenbreite
                        color: neon_green,     // Rahmenfarbe
                    },
                    ..Default::default()
                };

                // Text mit Rahmen anzeigen
                frame.show(ui, |ui| {
                    ui.label(egui::RichText::new(text).color(neon_green).monospace());
                });
            });
        });

        egui::TopBottomPanel::bottom("progress_panel").show(egui_context.ctx_mut(), |ui| {
            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::TopDown), |ui| {
                ui.add(
                    egui::ProgressBar::new(0.5)
                        .text(format!("{:.0}%", 0.5 * 100.0))
                        .desired_width(300.0), // Breite der Progressbar
                );
            });
        });

        egui::CentralPanel::default().frame(my_frame)
            .show(egui_context.ctx_mut(), |ui| {
                let mut style = (*ui.ctx().style()).clone();
                // Redefine text_styles
                style.text_styles = [
                    (egui::TextStyle::Heading, egui::FontId::new(30.0, egui::FontFamily::Proportional)),
                    (egui::TextStyle::Name("Heading2".into()), egui::FontId::new(25.0, egui::FontFamily::Proportional)),
                    (egui::TextStyle::Name("Context".into()), egui::FontId::new(23.0, egui::FontFamily::Proportional)),
                    (egui::TextStyle::Body, egui::FontId::new(24.0, egui::FontFamily::Proportional)),
                    (egui::TextStyle::Monospace, egui::FontId::new(14.0, egui::FontFamily::Monospace)),
                    (egui::TextStyle::Button, egui::FontId::new(24.0, egui::FontFamily::Proportional)),
                    (egui::TextStyle::Small, egui::FontId::new(10.0, egui::FontFamily::Proportional)),
                ].into();
                style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke{
                    color:egui::Color32::WHITE,
                    width: 5.0
                }  ;
                // Mutate global style with above changes
                ui.ctx().set_style(style);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let progress_bar = egui::ProgressBar::new(
                            actor.hit_points as f32/actor.max_hit_points as f32)
                            .text(format!("Health {}({})", actor.hit_points, actor.max_hit_points));
                        ui.add_sized([400.0, 40.0], progress_bar);
                        ui.allocate_space(egui::Vec2::new(20.0, 40.0));
                    });
                    /*if ship.win_or_lost == WinOrLostState::Lost {
                        ui.allocate_space(egui::Vec2::new(20.0, 200.0));
                        ui.add_sized([800.0, 40.0],egui::Label::new("You Lost!"));
                    }*/
                });
            });
    }

}