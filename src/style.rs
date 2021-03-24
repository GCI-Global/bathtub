pub mod style {
    use iced::{button, checkbox, container, pick_list, text_input};
    #[derive(Debug, Clone, Copy)]
    pub enum Theme {
        Blue,
        DarkBlue,
        BlueBorderOnly,
        BlueDisabled,
        BlueDisabledBright,
        TabSelected,
        Active,
        Disabled,
        Idle,
        Red,
        RedDisabled,
        Green,
        Yellow,
        YellowSelected,
        LightGray,
        LighterGray,
    }

    impl Theme {
        pub const ALL: [Theme; 16] = [
            Theme::Active,
            Theme::Disabled,
            Theme::Idle,
            Theme::Blue,
            Theme::DarkBlue,
            Theme::TabSelected,
            Theme::BlueBorderOnly,
            Theme::BlueDisabled,
            Theme::BlueDisabledBright,
            Theme::Red,
            Theme::RedDisabled,
            Theme::Green,
            Theme::Yellow,
            Theme::YellowSelected,
            Theme::LightGray,
            Theme::LighterGray,
        ];
    }

    impl Default for Theme {
        fn default() -> Theme {
            Theme::Idle
        }
    }

    impl From<Theme> for Box<dyn container::StyleSheet> {
        fn from(theme: Theme) -> Self {
            match theme {
                Theme::Active => yellow::Tooltip.into(),
                Theme::Red => red::Container.into(),
                Theme::Yellow => yellow::Container.into(),
                Theme::LightGray => light_gray::Container.into(),
                Theme::LighterGray => lighter_gray::Container.into(),
                _ => Default::default(),
            }
        }
    }

    impl From<Theme> for Box<dyn button::StyleSheet> {
        fn from(theme: Theme) -> Self {
            match theme {
                Theme::Blue => blue::Button.into(),
                Theme::DarkBlue => dark_blue::Button.into(),
                Theme::TabSelected => tab_selected::Button.into(),
                Theme::BlueDisabled => blue_disabled::Button.into(),
                Theme::BlueBorderOnly => blue_border_only::Button.into(),
                Theme::BlueDisabledBright => blue_disabled_bright::Button.into(),
                Theme::Red => red::Button.into(),
                Theme::RedDisabled => red_disabled::Button.into(),
                Theme::Green => green::Button.into(),
                Theme::Yellow => yellow::Button.into(),
                Theme::YellowSelected => yellow_selected::Button.into(),
                _ => Default::default(),
            }
        }
    }

    impl From<Theme> for Box<dyn text_input::StyleSheet> {
        fn from(theme: Theme) -> Self {
            match theme {
                Theme::Blue => blue::TextInput.into(),
                Theme::Red => red::TextInput.into(),
                _ => Default::default(),
            }
        }
    }

    impl From<Theme> for Box<dyn pick_list::StyleSheet> {
        fn from(theme: Theme) -> Self {
            match theme {
                Theme::Blue => blue::PickList.into(),
                _ => Default::default(),
            }
        }
    }

    impl From<Theme> for Box<dyn checkbox::StyleSheet> {
        fn from(theme: Theme) -> Self {
            match theme {
                Theme::Blue => blue::Checkbox.into(),
                _ => Default::default(),
            }
        }
    }

    mod blue {
        use iced::{button, checkbox, pick_list, text_input, Color};

        pub struct Button;

        impl button::StyleSheet for Button {
            fn active(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(37, 171, 236).into(),
                    text_color: Color::from_rgb8(255, 255, 255),
                    ..button::Style::default()
                }
            }
            fn hovered(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(47, 181, 246).into(),
                    text_color: Color::from_rgb8(255, 255, 255),
                    ..button::Style::default()
                }
            }
        }

        pub struct PickList;

        impl pick_list::StyleSheet for PickList {
            fn active(&self) -> pick_list::Style {
                pick_list::Style {
                    text_color: Color::from_rgb8(255, 255, 255),
                    background: Color::from_rgb8(37, 171, 236).into(),
                    icon_size: 0.55,
                    border_color: Color::from_rgb8(37, 171, 236),
                    ..pick_list::Style::default()
                }
            }

            fn hovered(&self) -> pick_list::Style {
                pick_list::Style {
                    text_color: Color::from_rgb8(255, 255, 255),
                    background: Color::from_rgb8(37, 171, 236).into(),
                    icon_size: 0.55,
                    border_color: Color::from_rgb8(37, 171, 236),
                    ..pick_list::Style::default()
                }
            }

            fn menu(&self) -> pick_list::Menu {
                pick_list::Menu {
                    text_color: Color::WHITE,
                    background: Color::from_rgb8(37, 171, 236).into(),
                    selected_text_color: Color::WHITE,
                    selected_background: Color::from_rgb8(47, 181, 246).into(),
                    ..pick_list::Menu::default()
                }
            }
        }

        pub struct TextInput;

        impl text_input::StyleSheet for TextInput {
            fn active(&self) -> text_input::Style {
                text_input::Style {
                    background: Color::from_rgb8(255, 255, 255).into(),
                    border_radius: 0.0,
                    border_width: 2.0,
                    border_color: Color::from_rgb8(37, 171, 236),
                }
            }

            fn focused(&self) -> text_input::Style {
                text_input::Style {
                    background: Color::from_rgb8(255, 255, 255).into(),
                    border_radius: 0.0,
                    border_width: 2.0,
                    border_color: Color::from_rgb8(47, 181, 246),
                }
            }

            fn hovered(&self) -> text_input::Style {
                text_input::Style {
                    background: Color::from_rgb8(255, 255, 255).into(),
                    border_radius: 0.0,
                    border_width: 2.0,
                    border_color: Color::from_rgb8(47, 181, 246),
                }
            }

            fn placeholder_color(&self) -> Color {
                Color::from_rgb(0.6, 0.6, 0.6)
            }

            fn value_color(&self) -> Color {
                Color::BLACK
            }

            fn selection_color(&self) -> Color {
                Color::from_rgb8(0, 80, 238)
            }
        }

        pub struct Checkbox;

        impl checkbox::StyleSheet for Checkbox {
            fn active(&self, is_checked: bool) -> checkbox::Style {
                checkbox::Style {
                    background: if is_checked {
                        Color::from_rgb8(37, 171, 236).into()
                    } else {
                        Color::WHITE.into()
                    },
                    checkmark_color: Color::from_rgb8(255, 255, 255),
                    border_color: Color::from_rgb8(37, 171, 236),
                    border_width: 2.0,
                    border_radius: 0.0,
                }
            }
            fn hovered(&self, is_checked: bool) -> checkbox::Style {
                checkbox::Style {
                    background: if is_checked {
                        Color::from_rgb8(47, 181, 246).into()
                    } else {
                        Color::WHITE.into()
                    },
                    checkmark_color: Color::from_rgb8(255, 255, 255),
                    border_color: Color::from_rgb8(47, 181, 246),
                    border_width: 2.0,
                    border_radius: 0.0,
                }
            }
        }
    }
    mod dark_blue {
        use iced::{button, Color};

        pub struct Button;

        impl button::StyleSheet for Button {
            fn active(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(17, 151, 226).into(),
                    text_color: Color::from_rgb8(255, 255, 255),
                    ..button::Style::default()
                }
            }
            fn hovered(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(47, 181, 246).into(),
                    text_color: Color::from_rgb8(255, 255, 255),
                    ..button::Style::default()
                }
            }
        }
    }

    mod blue_disabled {
        use iced::{button, Color};

        pub struct Button;

        impl button::StyleSheet for Button {
            fn active(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(112, 128, 164).into(),
                    text_color: Color::from_rgb8(100, 100, 120),
                    ..button::Style::default()
                }
            }
            fn hovered(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(112, 128, 164).into(),
                    text_color: Color::from_rgb8(100, 100, 120),
                    ..button::Style::default()
                }
            }
        }
    }

    mod blue_disabled_bright {
        use iced::{button, Color};

        pub struct Button;

        impl button::StyleSheet for Button {
            fn active(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(162, 168, 214).into(),
                    text_color: Color::from_rgb8(150, 150, 170),
                    ..button::Style::default()
                }
            }
            fn hovered(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(162, 168, 214).into(),
                    text_color: Color::from_rgb8(150, 150, 170),
                    ..button::Style::default()
                }
            }
        }
    }

    mod tab_selected {
        use iced::{button, Color};

        pub struct Button;

        impl button::StyleSheet for Button {
            fn active(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(255, 255, 255).into(),
                    text_color: Color::from_rgb8(37, 171, 236),
                    ..button::Style::default()
                }
            }
            fn hovered(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(255, 255, 255).into(),
                    text_color: Color::from_rgb8(47, 171, 236),
                    ..button::Style::default()
                }
            }
        }
    }

    mod blue_border_only {
        use iced::{button, Color};

        pub struct Button;

        impl button::StyleSheet for Button {
            fn active(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(255, 255, 255).into(),
                    text_color: Color::from_rgb8(37, 171, 236),
                    border_color: Color::from_rgb8(37, 171, 236).into(),
                    border_width: 2.0,
                    ..button::Style::default()
                }
            }
            fn hovered(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(255, 255, 255).into(),
                    text_color: Color::from_rgb8(47, 171, 236),
                    border_color: Color::from_rgb8(47, 171, 236).into(),
                    border_width: 2.0,
                    ..button::Style::default()
                }
            }
        }
    }

    mod green {
        use iced::{button, Color};

        pub struct Button;

        impl button::StyleSheet for Button {
            fn active(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(96, 196, 23).into(),
                    text_color: Color::from_rgb8(255, 255, 255),
                    ..button::Style::default()
                }
            }
            fn hovered(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(116, 216, 43).into(),
                    text_color: Color::from_rgb8(255, 255, 255),
                    ..button::Style::default()
                }
            }
        }
    }

    mod red {
        use iced::{button, container, text_input, Color};

        pub struct Container;

        impl container::StyleSheet for Container {
            fn style(&self) -> container::Style {
                container::Style {
                    text_color: Some(Color::WHITE),
                    background: Color::from_rgb8(249, 40, 20).into(),
                    border_color: Color::from_rgb8(229, 20, 0),
                    border_width: 5.0,
                    border_radius: 8.0,
                }
            }
        }

        pub struct Button;

        impl button::StyleSheet for Button {
            fn active(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(229, 20, 0).into(),
                    text_color: Color::from_rgb8(255, 255, 255),
                    ..button::Style::default()
                }
            }
            fn hovered(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(249, 40, 20).into(),
                    text_color: Color::from_rgb8(255, 255, 255),
                    ..button::Style::default()
                }
            }
        }

        pub struct TextInput;

        impl text_input::StyleSheet for TextInput {
            fn active(&self) -> text_input::Style {
                text_input::Style {
                    background: Color::from_rgb8(255, 255, 255).into(),
                    border_radius: 0.0,
                    border_width: 2.0,
                    border_color: Color::from_rgb8(229, 20, 0),
                }
            }

            fn focused(&self) -> text_input::Style {
                text_input::Style {
                    background: Color::from_rgb8(255, 255, 255).into(),
                    border_radius: 0.0,
                    border_width: 2.0,
                    border_color: Color::from_rgb8(249, 40, 20),
                }
            }

            fn hovered(&self) -> text_input::Style {
                text_input::Style {
                    background: Color::from_rgb8(255, 255, 255).into(),
                    border_radius: 0.0,
                    border_width: 2.0,
                    border_color: Color::from_rgb8(249, 40, 20),
                }
            }

            fn placeholder_color(&self) -> Color {
                Color::from_rgb(0.4, 0.4, 0.4)
            }

            fn value_color(&self) -> Color {
                Color::BLACK
            }

            fn selection_color(&self) -> Color {
                Color::from_rgb8(249, 40, 20)
            }
        }
    }

    mod red_disabled {
        use iced::{button, Color};

        pub struct Button;

        impl button::StyleSheet for Button {
            fn active(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(164, 128, 112).into(),
                    text_color: Color::from_rgb8(120, 100, 100),
                    ..button::Style::default()
                }
            }
            fn hovered(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(164, 128, 112).into(),
                    text_color: Color::from_rgb8(120, 100, 100),
                    ..button::Style::default()
                }
            }
        }
    }

    mod yellow {
        use iced::{button, container, Color};

        pub struct Button;

        impl button::StyleSheet for Button {
            fn active(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(255, 191, 10).into(),
                    text_color: Color::BLACK,
                    ..button::Style::default()
                }
            }
            fn hovered(&self) -> button::Style {
                button::Style {
                    background: Color::from_rgb8(255, 211, 30).into(),
                    text_color: Color::BLACK,
                    ..button::Style::default()
                }
            }
        }

        pub struct Tooltip;

        impl container::StyleSheet for Tooltip {
            fn style(&self) -> container::Style {
                container::Style {
                    text_color: Some(Color::BLACK),
                    background: Some(Color::from_rgb8(255, 193, 10).into()),
                    border_radius: 6.0,
                    ..container::Style::default()
                }
            }
        }

        pub struct Container;

        impl container::StyleSheet for Container {
            fn style(&self) -> container::Style {
                container::Style {
                    text_color: Some(Color::BLACK),
                    background: Color::from_rgb8(255, 193, 10).into(),
                    border_color: Color::from_rgb8(245, 183, 0),
                    border_width: 5.0,
                    border_radius: 8.0,
                }
            }
        }
    }

    mod yellow_selected {
        use iced::{button, Color};

        pub struct Button;

        impl button::StyleSheet for Button {
            fn active(&self) -> button::Style {
                button::Style {
                    background: Color::WHITE.into(),
                    text_color: Color::from_rgb8(243, 156, 18),
                    ..button::Style::default()
                }
            }
            fn hovered(&self) -> button::Style {
                button::Style {
                    background: Color::WHITE.into(),
                    text_color: Color::from_rgb8(253, 166, 28),
                    ..button::Style::default()
                }
            }
        }
    }
    mod light_gray {
        use iced::{container, Color};

        pub struct Container;

        impl container::StyleSheet for Container {
            fn style(&self) -> container::Style {
                container::Style {
                    text_color: Some(Color::BLACK),
                    background: Color::from_rgb8(220, 220, 220).into(),
                    border_radius: 0.0,
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                }
            }
        }
    }
    mod lighter_gray {
        use iced::{container, Color};

        pub struct Container;

        impl container::StyleSheet for Container {
            fn style(&self) -> container::Style {
                container::Style {
                    text_color: Some(Color::BLACK),
                    background: Color::from_rgb8(240, 240, 240).into(),
                    border_radius: 0.0,
                    border_width: 0.0,
                    border_color: Color::TRANSPARENT,
                }
            }
        }
    }
}
