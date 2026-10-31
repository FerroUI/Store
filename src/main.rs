#![cfg_attr(target_os = "emscripten", no_main)]

use ferroui_base::input::InputElement;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{
    Color, FontManagerOptions, FontWeight, Geometry, IBrush, Stretch, TextWrapping,
};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{
    CornerRadius, FerroObjectImpl, Ref, Thickness, Visual, ferro_class, ferro_impl_classes,
    instantiate,
};
use ferroui_browser::{BrowserAccessibilityMode, BrowserAppBuilder, BrowserPlatformOptions};
use ferroui_controls::media::{
    ApplicationTheme, RevealBackgroundBrush, RevealBorderBrush, RevealBrush, RevealBrushState,
};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Border, Button,
    ColumnDefinitions, Control, Grid, Image, NewApplication, RowDefinitions, ScrollViewer,
    StackPanel, TextBlock, TextBox, WrapPanel,
};
use ferroui_fonts_inter::AppBuilderExtension;
use ferroui_themes_winui::WinUITheme;
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, io::Cursor, rc::Rc};
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Media {
    url: String,
    alt: String,
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
struct Launch {
    url: String,
    mode: String,
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
struct AppInfo {
    id: String,
    name: String,
    summary: String,
    description: String,
    category: String,
    author: String,
    icon: String,
    accent: String,
    launch: Launch,
    #[serde(default)]
    screenshots: Vec<Media>,
    #[serde(default)]
    videos: Vec<Media>,
    #[serde(default)]
    tags: Vec<String>,
    version: Option<String>,
    source: Option<String>,
}
#[derive(Default)]
struct Store {
    apps: Vec<AppInfo>,
    query: String,
    category: String,
    selected: Option<String>,
    width: f64,
    body: Option<Ref<Border>>,
    sidebar: Option<Ref<Border>>,
    grid: Option<Ref<Grid>>,
    images: HashMap<String, Rc<Bitmap>>,
    waiting: Vec<(String, ferroui_base::WeakRef<Image>)>,
    controls: Vec<(String, Ref<Button>)>,
    permanent_controls: usize,
    navigation: Vec<(String, Ref<Button>)>,
    searches: Vec<Ref<TextBox>>,
    mobile_top: Option<Ref<Border>>,
    mobile_bottom: Option<Ref<Border>>,
    scroll: Option<Ref<ScrollViewer>>,
    collection: bool,
    status: Option<Ref<TextBlock>>,
    last_status: String,
}
thread_local! { static STORE: RefCell<Store> = RefCell::new(Store::default()); }

#[wasm_bindgen(
    inline_js = "export function storeAction(action, payload) { globalThis.ferroStoreAction?.(action, payload); }"
)]
extern "C" {
    #[wasm_bindgen(js_name = storeAction)]
    fn host(action: &str, payload: &str);
}

#[repr(C)]
pub struct App {
    base: Application,
}
ferro_class!(App: Application);
ferro_impl_classes!(App: FerroObjectImpl);
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        this.set_name(Some("Ferro Store".into()));
        this.set_requested_theme_variant(Some(ThemeVariant::light()));
        this.styles().add(WinUITheme::new().as_style());
    }
    fn on_framework_initialization_completed(this: &Self) {
        if let Some(lifetime) = this
            .application_lifetime()
            .as_ref()
            .and_then(|l| l.as_single_view_application_lifetime())
        {
            lifetime.set_main_view(Some(shell()));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}
fn color(s: &str) -> Color {
    Color::parse(s).unwrap()
}
fn brush(s: &str) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color(s)))
}
fn label(s: &str, size: f64, ink: &str) -> Ref<TextBlock> {
    let t = TextBlock::new();
    t.set_text(Some(s));
    t.set_font_size(size);
    t.set_foreground(Some(brush(ink)));
    t.set_text_wrapping(TextWrapping::Wrap);
    t
}
fn heading(s: &str, size: f64) -> Ref<TextBlock> {
    let t = label(s, size, "#1D1D1F");
    t.set_font_weight(FontWeight::SemiBold);
    t
}
fn stack(gap: f64) -> Ref<StackPanel> {
    let p = StackPanel::new();
    p.set_spacing(gap);
    p
}
fn row(gap: f64) -> Ref<StackPanel> {
    let p = stack(gap);
    p.set_orientation(Orientation::Horizontal);
    p
}
fn surface(bg: &str, padding: f64, radius: f64) -> Ref<Border> {
    let b = Border::new();
    b.set_background(Some(brush(bg)));
    b.set_padding(Thickness::uniform(padding));
    b.set_corner_radius(CornerRadius::uniform(radius));
    b
}
fn gradient(b: &Ref<Border>, from: &str, to: &str) {
    let g = ferroui_base::media::LinearGradientBrush::new();
    g.gradient_stops()
        .add(ferroui_base::media::GradientStop::with_color_and_offset(
            color(from),
            0.0,
        ));
    g.gradient_stops()
        .add(ferroui_base::media::GradientStop::with_color_and_offset(
            color(to),
            1.0,
        ));
    b.set_background(Some(g.into()));
}
fn button(title: &str, primary: bool, f: impl Fn() + 'static) -> Ref<Button> {
    let b = Button::new();
    b.set_content(Some(Rc::new(title.to_owned())));
    b.set_padding(Thickness::new(16.0, 8.0, 16.0, 8.0));
    b.set_corner_radius(CornerRadius::uniform(16.0));
    b.set_background(Some(brush(if primary { "#007AFF" } else { "#F1F2F5" })));
    b.set_foreground(Some(brush(if primary { "#FFFFFF" } else { "#007AFF" })));
    b.set_border_thickness(Thickness::uniform(0.0));
    b.set_font_size(13.0);
    ferroui_controls::automation::AutomationProperties::set_name(&b, Some(title));
    b.click(move |_, _| f());
    STORE.with(|s| s.borrow_mut().controls.push((title.to_owned(), b.clone())));
    b
}
fn link(title: &str, f: impl Fn() + 'static) -> Ref<Button> {
    let b = button(title, false, f);
    b.set_background(Some(brush("Transparent")));
    b.set_padding(Thickness::uniform(0.0));
    b
}
fn glyph(name: &str, size: f64, ink: &str) -> Ref<ferroui_controls::shapes::Path> {
    let data = match name {
        "discover" => "M12 1 A11 11 0 1 1 11.99 1 M15.5 8.5 L13.5 13.5 L8.5 15.5 L10.5 10.5 Z",
        "create" => "M3 17 L2 22 L7 21 L21 7 L17 3 Z M14 6 L18 10",
        "work" => "M3 7 L21 7 L21 21 L3 21 Z M8 7 L8 3 L16 3 L16 7 M3 12 L21 12",
        "read" => {
            "M12 5 C8 2 4 2 2 3 L2 20 C6 19 9 20 12 22 C15 20 18 19 22 20 L22 3 C18 2 15 2 12 5 Z M12 5 L12 22"
        }
        "play" => {
            "M7 6 L17 6 C21 6 23 18 20 19 C18 20 16 15 15 15 L9 15 C8 15 6 20 3 19 C0 18 3 6 7 6 Z M5 10 L5 14 M3 12 L7 12 M16 10 L17 10 M19 13 L20 13"
        }
        "apps" => {
            "M2 2 L9 2 L9 9 L2 9 Z M15 2 L22 2 L22 9 L15 9 Z M2 15 L9 15 L9 22 L2 22 Z M15 15 L22 15 L22 22 L15 22 Z"
        }
        "refresh" => "M20 8 A9 9 0 1 0 21 15 M20 2 L20 8 L14 8",
        _ => "M3 12 L21 12 M14 5 L21 12 L14 19",
    };
    let p = ferroui_controls::shapes::Path::new();
    p.set_data(Geometry::parse(data).ok());
    p.set_stroke(Some(brush(ink)));
    p.set_stroke_thickness(1.65);
    p.set_width(size);
    p.set_height(size);
    p.set_stretch(Stretch::Uniform);
    p
}
fn nav_button(title: &str, category: &str, icon: &str, compact: bool) -> Ref<Button> {
    let cat = category.to_owned();
    let b = button(title, false, move || {
        if cat == "Apps" {
            browse_apps();
        } else {
            select_category(cat.clone());
        }
    });
    let content = if compact { stack(4.0) } else { row(12.0) };
    content
        .children()
        .add(glyph(icon, if compact { 19.0 } else { 20.0 }, "#007AFF").upcast::<Control>());
    let t = label(title, if compact { 10.0 } else { 14.0 }, "#45454D");
    t.set_vertical_alignment(VerticalAlignment::Center);
    content.children().add(t.upcast::<Control>());
    b.set_content(Some(Control::boxed(content)));
    b.set_background(Some(brush("Transparent")));
    b.set_corner_radius(CornerRadius::uniform(8.0));
    b.set_padding(Thickness::new(12.0, 11.0, 12.0, 11.0));
    b.set_horizontal_alignment(HorizontalAlignment::Stretch);
    b.set_horizontal_content_alignment(if compact {
        HorizontalAlignment::Center
    } else {
        HorizontalAlignment::Left
    });
    STORE.with(|s| {
        s.borrow_mut()
            .navigation
            .push((category.to_owned(), b.clone()))
    });
    b
}
fn reveal(b: &Ref<Border>) {
    let fill = RevealBackgroundBrush::new();
    fill.set_color(color("#02007AFF"));
    fill.set_fallback_color(color("Transparent"));
    fill.set_target_theme(ApplicationTheme::Light);
    let edge = RevealBorderBrush::new();
    edge.set_color(color("#08007AFF"));
    edge.set_fallback_color(color("Transparent"));
    edge.set_target_theme(ApplicationTheme::Light);
    b.set_background(Some(fill.into()));
    b.set_border_brush(Some(edge.into()));
    b.set_border_thickness(Thickness::uniform(1.0));
    macro_rules! follow {
        ($event:expr,$state:expr) => {{
            let weak = b.downgrade();
            b.add_handler($event, move |_, _| {
                if let Some(b) = weak.upgrade() {
                    RevealBrush::set_state(&b, $state);
                }
            });
        }};
    }
    follow!(
        InputElement::pointer_entered_event(),
        RevealBrushState::PointerOver
    );
    follow!(
        InputElement::pointer_exited_event(),
        RevealBrushState::Normal
    );
    follow!(
        InputElement::pointer_pressed_event(),
        RevealBrushState::Pressed
    );
    follow!(
        InputElement::pointer_released_event(),
        RevealBrushState::PointerOver
    );
}
fn image(url: &str, w: f64, h: f64) -> Ref<Image> {
    let i = Image::new();
    i.set_width(w);
    i.set_height(h);
    i.set_stretch(Stretch::Uniform);
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(bitmap) = s.images.get(url) {
            i.set_source(Some(bitmap.clone()));
        }
        s.waiting.push((url.into(), i.downgrade()));
    });
    i
}
fn open_details(id: String) {
    host("navigate", &id);
}
fn select_category(category: String) {
    let searches = STORE.with(|s| {
        let mut s = s.borrow_mut();
        let reset = category == "All apps";
        s.category = category;
        s.collection = false;
        s.selected = None;
        if reset {
            s.query.clear();
            s.searches.clone()
        } else {
            Vec::new()
        }
    });
    for search in searches {
        search.set_text(None);
    }
    host("navigate", ""); // Keep the browser route consistent when leaving an app detail.
}
fn browse_apps() {
    let searches = STORE.with(|s| {
        let mut s = s.borrow_mut();
        s.category = "All apps".into();
        s.collection = true;
        s.selected = None;
        s.query.clear();
        s.searches.clone()
    });
    for search in searches {
        search.set_text(None);
    }
    host("navigate", "");
}
fn search_box() -> Ref<TextBox> {
    let search = TextBox::new();
    search.set_placeholder_text(Some("Search apps, ideas…"));
    search.set_font_size(12.0);
    search.set_background(Some(brush("#ECEEF2")));
    search.set_border_thickness(Thickness::uniform(0.0));
    search.set_corner_radius(CornerRadius::uniform(9.0));
    search.set_padding(Thickness::new(12.0, 9.0, 12.0, 9.0));
    ferroui_controls::automation::AutomationProperties::set_name(&search, Some("Search apps"));
    let weak = search.downgrade();
    search.text_changed(move |_, _| {
        let value = weak.upgrade().and_then(|t| t.text()).unwrap_or_default();
        let others = STORE.with(|s| {
            let mut s = s.borrow_mut();
            if s.query == value {
                return None;
            }
            s.query = value.clone();
            s.selected = None;
            s.category = "All apps".into();
            s.collection = false;
            Some(s.searches.clone())
        });
        if let Some(others) = others {
            host("search", "");
            for other in others {
                if other.text().as_deref() != Some(&value) {
                    other.set_text(Some(&value));
                }
            }
            render();
        }
    });
    STORE.with(|s| s.borrow_mut().searches.push(search.clone()));
    search
}
fn brand(compact: bool) -> Ref<StackPanel> {
    let logo = row(9.0);
    let mark = surface("#007AFF", 0.0, 8.0);
    mark.set_width(if compact { 31.0 } else { 35.0 });
    mark.set_height(if compact { 31.0 } else { 35.0 });
    let f = label("F", 24.0, "#FFFFFF");
    f.set_horizontal_alignment(HorizontalAlignment::Center);
    f.set_vertical_alignment(VerticalAlignment::Center);
    f.set_font_weight(FontWeight::SemiBold);
    mark.set_child(f);
    logo.children().add(mark.upcast::<Control>());
    let title = heading("Ferro Store", 17.0);
    title.set_vertical_alignment(VerticalAlignment::Center);
    logo.children().add(title.upcast::<Control>());
    logo
}
fn shell() -> Ref<Control> {
    let root = Grid::new();
    root.set_column_definitions("220,*".parse::<ColumnDefinitions>().unwrap());
    root.set_background(Some(brush("#FFFFFF")));
    let side = surface("#F3F4F7", 16.0, 0.0);
    let layout = Grid::new();
    layout.set_row_definitions("Auto,* ,Auto".parse::<RowDefinitions>().unwrap());
    let top = stack(24.0);
    let logo = brand(false);
    logo.set_margin(Thickness::new(0.0, 8.0, 0.0, 0.0));
    top.children().add(logo.upcast::<Control>());
    top.children().add(search_box().upcast::<Control>());
    let nav = stack(6.0);
    for (title, category, icon) in [
        ("Discover", "All apps", "discover"),
        ("Create", "Design", "create"),
        ("Work", "Productivity", "work"),
        ("Read", "News", "read"),
        ("Play", "Games", "play"),
    ] {
        nav.children()
            .add(nav_button(title, category, icon, false).upcast::<Control>());
    }
    top.children().add(nav.upcast::<Control>());
    layout.children().add(top.upcast::<Control>());
    let built = link("Built with FerroUI  ↗", || host("framework", ""));
    built.set_font_size(11.0);
    built.set_foreground(Some(brush("#6E6E73")));
    built.set_margin(Thickness::new(8.0, 20.0, 0.0, 8.0));
    Grid::set_row(&built, 2);
    layout.children().add(built.upcast::<Control>());
    side.set_child(layout);
    root.children().add(side.clone().upcast::<Control>());
    let main = Grid::new();
    main.set_row_definitions("Auto,*,Auto".parse::<RowDefinitions>().unwrap());
    Grid::set_column(&main, 1);
    let mobile_top = surface("#FFFFFF", 0.0, 0.0);
    let mobile_header = stack(16.0);
    mobile_header.set_margin(Thickness::new(20.0, 18.0, 20.0, 0.0));
    mobile_header
        .children()
        .add(brand(true).upcast::<Control>());
    mobile_header
        .children()
        .add(search_box().upcast::<Control>());
    mobile_top.set_child(mobile_header);
    main.children().add(mobile_top.clone().upcast::<Control>());
    let body = Border::new();
    let scroll = ScrollViewer::new();
    scroll.set_horizontal_scroll_bar_visibility(
        ferroui_controls::primitives::ScrollBarVisibility::Disabled,
    );
    scroll.set_content(Some(Control::boxed(body.clone())));
    Grid::set_row(&scroll, 1);
    main.children().add(scroll.clone().upcast::<Control>());
    let bottom = surface("#FAFAFC", 0.0, 0.0);
    bottom.set_border_brush(Some(brush("#E3E3E8")));
    bottom.set_border_thickness(Thickness::new(0.0, 1.0, 0.0, 0.0));
    let tabs = Grid::new();
    tabs.set_column_definitions("*,*,*".parse::<ColumnDefinitions>().unwrap());
    for (i, (title, category, icon)) in [
        ("Discover", "All apps", "discover"),
        ("Apps", "Apps", "apps"),
        ("Play", "Games", "play"),
    ]
    .into_iter()
    .enumerate()
    {
        let b = nav_button(title, category, icon, true);
        Grid::set_column(&b, i as i32);
        tabs.children().add(b.upcast::<Control>());
    }
    bottom.set_child(tabs);
    Grid::set_row(&bottom, 2);
    main.children().add(bottom.clone().upcast::<Control>());
    root.children().add(main.upcast::<Control>());
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        s.body = Some(body);
        s.sidebar = Some(side);
        s.grid = Some(root.clone());
        s.mobile_top = Some(mobile_top);
        s.mobile_bottom = Some(bottom);
        s.scroll = Some(scroll);
        s.permanent_controls = s.controls.len();
    });
    let width = STORE.with(|s| s.borrow().width);
    update_shell(width);
    render();
    root.upcast::<Control>()
}
fn update_shell(width: f64) {
    let (grid, side, top, bottom) = STORE.with(|s| {
        let s = s.borrow();
        (
            s.grid.clone(),
            s.sidebar.clone(),
            s.mobile_top.clone(),
            s.mobile_bottom.clone(),
        )
    });
    let mobile = width < 900.0;
    if let Some(g) = grid {
        g.set_column_definitions(
            if mobile { "0,*" } else { "220,*" }
                .parse::<ColumnDefinitions>()
                .unwrap(),
        );
    }
    if let Some(s) = side {
        s.set_is_visible(!mobile);
    }
    if let Some(t) = top {
        t.set_is_visible(mobile);
    }
    if let Some(b) = bottom {
        b.set_is_visible(mobile);
    }
}
fn preview(app: &AppInfo, w: f64, h: f64, tilt: f64, crop: bool) -> Ref<Border> {
    let b = surface("#FFFFFF", 0.0, 10.0);
    b.set_width(w);
    b.set_height(h);
    b.set_clip_to_bounds(true);
    if let Some(screenshot) = app.screenshots.first() {
        let i = image(&screenshot.url, w, h);
        i.set_stretch(if crop {
            Stretch::UniformToFill
        } else {
            Stretch::Uniform
        });
        b.set_child(i);
    } else {
        b.set_child(image(&app.icon, w.min(80.0), h.min(80.0)));
    }
    if tilt != 0.0 {
        b.set_render_transform(Some(
            ferroui_base::media::RotateTransform::with_angle(tilt).into(),
        ));
    }
    if let Ok(shadow) = ferroui_base::media::BoxShadows::parse("0 10 28 0 #22000000") {
        b.set_box_shadow(shadow);
    }
    b
}
fn feature(app: &AppInfo, w: f64, mobile: bool, play: bool) -> Ref<Border> {
    let b = surface(if play { "#0C693D" } else { "#E7E0FC" }, 0.0, 14.0);
    gradient(
        &b,
        if play { "#176E46" } else { "#EAE5FF" },
        if play { "#03552F" } else { "#DCD2FC" },
    );
    b.set_width(w);
    b.set_height(if mobile { 374.0 } else { 342.0 });
    b.set_clip_to_bounds(true);
    let layout = Grid::new();
    let horizontal = !mobile && !play;
    layout.set_column_definitions(
        if horizontal { "49*,51*" } else { "*" }
            .parse::<ColumnDefinitions>()
            .unwrap(),
    );
    if !horizontal {
        layout.set_row_definitions("Auto,*".parse::<RowDefinitions>().unwrap());
    }
    let copy = stack(12.0);
    copy.set_margin(Thickness::new(
        24.0,
        26.0,
        if horizontal { 0.0 } else { 24.0 },
        0.0,
    ));
    copy.children().add(
        label(
            if play {
                "TAKE A BREAK"
            } else {
                "MAKE SOMETHING GREAT"
            },
            10.0,
            if play { "#D6F0DF" } else { "#34416D" },
        )
        .upcast::<Control>(),
    );
    let headline = label(
        if play {
            "A little time\nto play."
        } else {
            "Make room\nfor your next idea."
        },
        if mobile {
            26.0
        } else if horizontal {
            30.0
        } else {
            28.0
        },
        if play { "#FFFFFF" } else { "#191624" },
    );
    headline.set_font_weight(FontWeight::SemiBold);
    copy.children().add(headline.upcast::<Control>());
    if !mobile {
        copy.children().add(
            label(
                if play {
                    "Solitaire is a classic,\nright in your browser."
                } else {
                    "A native design workspace,\nfrom first sketch to final idea."
                },
                13.0,
                if play { "#E1F0E5" } else { "#5F6075" },
            )
            .upcast::<Control>(),
        );
    }
    let id = app.id.clone();
    let json = serde_json::to_string(app).unwrap();
    let action = button(
        &format!("{} {}  →", if play { "Open" } else { "Explore" }, app.name),
        false,
        move || {
            if play {
                host("launch", &json);
            } else {
                open_details(id.clone());
            }
        },
    );
    action.set_background(Some(brush("#FFFFFF")));
    action.set_horizontal_alignment(HorizontalAlignment::Left);
    copy.children().add(action.upcast::<Control>());
    layout.children().add(copy.upcast::<Control>());
    let screen = preview(
        app,
        if horizontal { w * 0.64 } else { w + 5.0 },
        if horizontal { 305.0 } else { 190.0 },
        if play { -7.0 } else { -6.0 },
        true,
    );
    screen.set_horizontal_alignment(HorizontalAlignment::Left);
    screen.set_vertical_alignment(VerticalAlignment::Bottom);
    if horizontal {
        Grid::set_column(&screen, 1);
        screen.set_margin(Thickness::new(10.0, 65.0, -100.0, -29.0));
        layout.children().add(screen.upcast::<Control>());
    } else {
        let clip = Border::new();
        clip.set_clip_to_bounds(true);
        Grid::set_row(&clip, 1);
        screen.set_vertical_alignment(VerticalAlignment::Top);
        screen.set_margin(Thickness::new(24.0, 16.0, -60.0, -35.0));
        clip.set_child(screen);
        layout.children().add(clip.upcast::<Control>());
    }
    b.set_child(layout);
    b
}
fn app_row(app: &AppInfo, w: f64) -> Ref<Border> {
    let outer = surface("Transparent", 0.0, 9.0);
    outer.set_width(w);
    reveal(&outer);
    let all = stack(0.0);
    let g = Grid::new();
    g.set_column_definitions("64,*,Auto".parse::<ColumnDefinitions>().unwrap());
    g.set_column_spacing(8.0);
    g.set_min_height(92.0);
    g.set_margin(Thickness::new(8.0, 0.0, 8.0, 0.0));
    let icon = image(&app.icon, 56.0, 56.0);
    icon.set_vertical_alignment(VerticalAlignment::Center);
    g.children().add(icon.upcast::<Control>());
    let identity = stack(5.0);
    identity.set_vertical_alignment(VerticalAlignment::Center);
    let id = app.id.clone();
    let title = link(&app.name, move || open_details(id.clone()));
    title.set_foreground(Some(brush("#1D1D1F")));
    title.set_font_size(15.0);
    title.set_font_weight(FontWeight::SemiBold);
    title.set_horizontal_alignment(HorizontalAlignment::Left);
    identity.children().add(title.upcast::<Control>());
    let summary = label(&app.summary, 12.0, "#6E6E73");
    summary.set_max_height(32.0);
    summary.set_clip_to_bounds(true);
    identity.children().add(summary.upcast::<Control>());
    Grid::set_column(&identity, 1);
    g.children().add(identity.upcast::<Control>());
    let json = serde_json::to_string(app).unwrap();
    let open = button("OPEN", false, move || host("launch", &json));
    open.set_padding(Thickness::new(15.0, 6.0, 15.0, 6.0));
    open.set_font_size(11.0);
    open.set_font_weight(FontWeight::SemiBold);
    open.set_vertical_alignment(VerticalAlignment::Center);
    Grid::set_column(&open, 2);
    g.children().add(open.upcast::<Control>());
    all.children().add(g.upcast::<Control>());
    let divider = surface("#E9E9EE", 0.0, 0.0);
    divider.set_height(1.0);
    all.children().add(divider.upcast::<Control>());
    outer.set_child(all);
    outer
}
fn everyday(app: &AppInfo, w: f64) -> Ref<Border> {
    let calc = app.id == "calculator";
    let b = surface(if calc { "#E7F1FC" } else { "#FDE9E0" }, 0.0, 13.0);
    gradient(
        &b,
        if calc { "#ECF5FE" } else { "#FFF1E9" },
        if calc { "#DCEBFA" } else { "#FADCD0" },
    );
    b.set_width(w);
    b.set_height(224.0);
    b.set_clip_to_bounds(true);
    let g = Grid::new();
    g.set_column_definitions("52*,48*".parse::<ColumnDefinitions>().unwrap());
    let copy = stack(11.0);
    copy.set_margin(Thickness::new(22.0, 23.0, 0.0, 16.0));
    copy.children().add(
        label(
            if calc {
                "DO MORE WITH NUMBERS"
            } else {
                "GOOD STORIES. BETTER CONVERSATIONS."
            },
            9.0,
            "#34416D",
        )
        .upcast::<Control>(),
    );
    copy.children().add(
        heading(
            if calc {
                "A little more\nclarity."
            } else {
                "A little more\ncuriosity."
            },
            25.0,
        )
        .upcast::<Control>(),
    );
    if w > 420.0 {
        copy.children().add(
            label(
                if calc {
                    "Keep the essentials close at hand."
                } else {
                    "Explore what's happening."
                },
                12.0,
                "#62677A",
            )
            .upcast::<Control>(),
        );
    }
    let json = serde_json::to_string(app).unwrap();
    let open = button(&format!("Open {}  →", app.name), false, move || {
        host("launch", &json)
    });
    open.set_background(Some(brush("#FFFFFF")));
    open.set_horizontal_alignment(HorizontalAlignment::Left);
    copy.children().add(open.upcast::<Control>());
    g.children().add(copy.upcast::<Control>());
    let screen = preview(
        app,
        if calc { w * 0.48 } else { w * 0.88 },
        if calc { w * 0.9143 } else { w * 0.66 },
        0.0,
        false,
    );
    screen.set_vertical_alignment(VerticalAlignment::Top);
    screen.set_horizontal_alignment(HorizontalAlignment::Left);
    screen.set_margin(Thickness::new(0.0, 20.0, -200.0, -200.0));
    Grid::set_column(&screen, 1);
    g.children().add(screen.upcast::<Control>());
    b.set_child(g);
    b
}
fn render() {
    let (apps, query, category, selected, width, body, collection, permanent) = STORE.with(|s| {
        let s = s.borrow();
        (
            s.apps.clone(),
            s.query.clone(),
            s.category.clone(),
            s.selected.clone(),
            s.width,
            s.body.clone(),
            s.collection,
            s.permanent_controls,
        )
    });
    let Some(body) = body else {
        return;
    };
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        s.controls.truncate(permanent);
        s.waiting.retain(|(_, w)| w.upgrade().is_some());
        for (key, b) in &s.navigation {
            let active = if collection {
                key == "Apps"
            } else {
                *key == category
            };
            b.set_background(Some(brush(if active { "#DEEBFC" } else { "Transparent" })));
        }
    });
    let mobile = width < 900.0;
    let margin = if mobile { 20.0 } else { 28.0 };
    let available = (width - if mobile { 0.0 } else { 220.0 } - margin * 2.0).clamp(240.0, 1240.0);
    let p = stack(22.0);
    p.set_max_width(1240.0);
    p.set_margin(Thickness::new(
        margin,
        if mobile { 20.0 } else { 28.0 },
        margin,
        28.0,
    ));
    if let Some(app) = selected
        .as_ref()
        .and_then(|id| apps.iter().find(|a| &a.id == id))
    {
        details(&p, app, available);
    } else {
        let q = query.to_lowercase();
        let visible: Vec<_> = apps
            .iter()
            .filter(|a| {
                (category == "All apps" || a.category == category)
                    && format!(
                        "{} {} {} {}",
                        a.name,
                        a.summary,
                        a.category,
                        a.tags.join(" ")
                    )
                    .to_lowercase()
                    .contains(&q)
            })
            .collect();
        let discover = query.is_empty() && category == "All apps" && !collection;
        let header = Grid::new();
        header.set_column_definitions("*,Auto".parse::<ColumnDefinitions>().unwrap());
        let title = stack(7.0);
        title.children().add(
            heading(
                if !query.is_empty() {
                    "Search"
                } else if discover {
                    "Discover"
                } else if collection {
                    "Apps"
                } else {
                    match category.as_str() {
                        "Design" => "Create",
                        "Productivity" => "Work",
                        "News" => "Read",
                        "Games" => "Play",
                        _ => &category,
                    }
                },
                32.0,
            )
            .upcast::<Control>(),
        );
        title.children().add(
            label(
                if discover {
                    "Great apps, built with Ferro."
                } else if !query.is_empty() {
                    "Find something that fits your day."
                } else {
                    "Explore the Ferro collection."
                },
                14.0,
                "#6E6E73",
            )
            .upcast::<Control>(),
        );
        header.children().add(title.upcast::<Control>());
        let refresh = button("Refresh", false, || host("refresh", ""));
        refresh.set_content(Some(Control::boxed(glyph("refresh", 18.0, "#6E6E73"))));
        refresh.set_padding(Thickness::uniform(9.0));
        refresh.set_corner_radius(CornerRadius::uniform(9.0));
        refresh.set_background(Some(brush("#F8F8FA")));
        refresh.set_vertical_alignment(VerticalAlignment::Top);
        Grid::set_column(&refresh, 1);
        header.children().add(refresh.upcast::<Control>());
        p.children().add(header.upcast::<Control>());
        if discover {
            let design = apps.iter().find(|a| a.id == "ferrodesign").or(apps.first());
            let solitaire = apps.iter().find(|a| a.id == "solitaire");
            if let Some(design) = design {
                if mobile {
                    let rail = row(14.0);
                    rail.children()
                        .add(feature(design, available - 18.0, true, false).upcast::<Control>());
                    if let Some(game) = solitaire {
                        rail.children()
                            .add(feature(game, available - 18.0, true, true).upcast::<Control>());
                    }
                    let scroll = ScrollViewer::new();
                    scroll.set_width(available);
                    scroll.set_height(374.0);
                    scroll.set_horizontal_scroll_bar_visibility(
                        ferroui_controls::primitives::ScrollBarVisibility::Hidden,
                    );
                    scroll.set_vertical_scroll_bar_visibility(
                        ferroui_controls::primitives::ScrollBarVisibility::Disabled,
                    );
                    scroll.set_content(Some(Control::boxed(rail)));
                    p.children().add(scroll.upcast::<Control>());
                } else {
                    let rail = row(16.0);
                    let main_w = if solitaire.is_some() {
                        ((available - 16.0) * 0.68).floor()
                    } else {
                        available
                    };
                    rail.children()
                        .add(feature(design, main_w, false, false).upcast::<Control>());
                    if let Some(game) = solitaire {
                        rail.children().add(
                            feature(game, available - main_w - 16.0, false, true)
                                .upcast::<Control>(),
                        );
                    }
                    p.children().add(rail.upcast::<Control>());
                }
            }
        }
        if !discover {
            let filters = WrapPanel::new();
            filters.set_item_spacing(6.0);
            filters.set_line_spacing(6.0);
            for (c, title) in [
                ("All apps", "All apps"),
                ("Productivity", "Work"),
                ("Design", "Create"),
                ("News", "Read"),
                ("Games", "Play"),
            ] {
                let cat = c.to_owned();
                let b = button(title, false, move || select_category(cat.clone()));
                if category == c {
                    b.set_background(Some(brush("#DEEBFC")));
                }
                b.set_padding(Thickness::new(12.0, 6.0, 12.0, 6.0));
                filters.children().add(b.upcast::<Control>());
            }
            if !discover {
                p.children().add(filters.upcast::<Control>());
            }
        }
        let section = Grid::new();
        section.set_column_definitions("*,Auto".parse::<ColumnDefinitions>().unwrap());
        section.children().add(
            heading(
                if query.is_empty() {
                    "Made with Ferro"
                } else {
                    "Search results"
                },
                22.0,
            )
            .upcast::<Control>(),
        );
        if discover {
            let all = link("Explore all  →", browse_apps);
            all.set_font_size(12.0);
            all.set_vertical_alignment(VerticalAlignment::Center);
            Grid::set_column(&all, 1);
            section.children().add(all.upcast::<Control>());
        }
        p.children().add(section.upcast::<Control>());
        let list = WrapPanel::new();
        list.set_item_spacing(24.0);
        list.set_line_spacing(0.0);
        let cols = if available >= 660.0 { 2.0 } else { 1.0 };
        let w = ((available - (cols - 1.0) * 24.0) / cols).floor();
        for app in &visible {
            list.children().add(app_row(app, w).upcast::<Control>());
        }
        p.children().add(list.upcast::<Control>());
        if visible.is_empty() {
            p.children().add(
                label(
                    "No apps found. Try another search or category.",
                    14.0,
                    "#6E6E73",
                )
                .upcast::<Control>(),
            );
        }
        if discover {
            let everyday_apps: Vec<_> = ["calculator", "ember"]
                .iter()
                .filter_map(|id| apps.iter().find(|a| a.id == *id))
                .collect();
            if !everyday_apps.is_empty() {
                p.children()
                    .add(heading("For your everyday", 22.0).upcast::<Control>());
                let stories = WrapPanel::new();
                stories.set_item_spacing(18.0);
                stories.set_line_spacing(18.0);
                let w = if available >= 660.0 {
                    ((available - 18.0) / 2.0).floor()
                } else {
                    available
                };
                for app in everyday_apps {
                    stories.children().add(everyday(app, w).upcast::<Control>());
                }
                p.children().add(stories.upcast::<Control>());
            }
        }
    }
    let status = STORE.with(|s| s.borrow().last_status.clone());
    let status = label(
        if status.is_empty() {
            "The Ferro collection"
        } else {
            &status
        },
        10.0,
        "#8A8A92",
    );
    STORE.with(|s| s.borrow_mut().status = Some(status.clone()));
    p.children().add(status.upcast::<Control>());
    body.set_child(p);
}
fn details(p: &Ref<StackPanel>, app: &AppInfo, available: f64) {
    let back = link("←  Back to apps", || host("navigate", ""));
    back.set_horizontal_alignment(HorizontalAlignment::Left);
    p.children().add(back.upcast::<Control>());
    let identity = Grid::new();
    identity.set_column_definitions("Auto,*".parse::<ColumnDefinitions>().unwrap());
    identity.set_column_spacing(20.0);
    identity.children().add(
        image(
            &app.icon,
            if available < 500.0 { 76.0 } else { 96.0 },
            if available < 500.0 { 76.0 } else { 96.0 },
        )
        .upcast::<Control>(),
    );
    let titles = stack(7.0);
    titles
        .children()
        .add(heading(&app.name, if available < 500.0 { 27.0 } else { 34.0 }).upcast::<Control>());
    titles
        .children()
        .add(label(&app.summary, 15.0, "#6E6E73").upcast::<Control>());
    titles
        .children()
        .add(label(&app.author, 12.0, "#007AFF").upcast::<Control>());
    Grid::set_column(&titles, 1);
    identity.children().add(titles.upcast::<Control>());
    p.children().add(identity.upcast::<Control>());
    let actions = WrapPanel::new();
    actions.set_item_spacing(12.0);
    actions.set_line_spacing(8.0);
    let json = serde_json::to_string(app).unwrap();
    let j = json.clone();
    actions
        .children()
        .add(button("Open app", true, move || host("launch", &j)).upcast::<Control>());
    actions
        .children()
        .add(button("Open in new tab  ↗", false, move || host("tab", &json)).upcast::<Control>());
    p.children().add(actions.upcast::<Control>());
    let info = surface("#F7F7FA", 16.0, 12.0);
    let fields = WrapPanel::new();
    fields.set_item_spacing(36.0);
    fields.set_line_spacing(14.0);
    for (title, value) in [
        ("CATEGORY", app.category.as_str()),
        ("DEVELOPER", app.author.as_str()),
        ("RUNS IN", "Your browser"),
        ("VERSION", app.version.as_deref().unwrap_or("Web app")),
    ] {
        let f = stack(5.0);
        f.children()
            .add(label(title, 9.0, "#8A8A92").upcast::<Control>());
        f.children()
            .add(label(value, 13.0, "#45454D").upcast::<Control>());
        fields.children().add(f.upcast::<Control>());
    }
    info.set_child(fields);
    p.children().add(info.upcast::<Control>());
    if !app.screenshots.is_empty() {
        p.children()
            .add(heading("Preview", 22.0).upcast::<Control>());
        let shot = preview(
            app,
            available.min(1000.0),
            if available < 500.0 { 220.0 } else { 400.0 },
            0.0,
            false,
        );
        shot.set_horizontal_alignment(HorizontalAlignment::Left);
        p.children().add(shot.upcast::<Control>());
        let json = serde_json::to_string(app).unwrap();
        p.children().add(
            link("View screenshots & video", move || host("media", &json)).upcast::<Control>(),
        );
    } else if !app.videos.is_empty() {
        let json = serde_json::to_string(app).unwrap();
        p.children()
            .add(button("Watch demo", false, move || host("media", &json)).upcast::<Control>());
    }
    p.children()
        .add(heading("About this app", 22.0).upcast::<Control>());
    let description = label(&app.description, 15.0, "#515159");
    description.set_max_width(820.0);
    description.set_horizontal_alignment(HorizontalAlignment::Left);
    p.children().add(description.upcast::<Control>());
    if !app.tags.is_empty() {
        p.children()
            .add(label(&app.tags.join("   ·   "), 12.0, "#86868B").upcast::<Control>());
    }
    if let Some(source) = &app.source {
        let source = source.clone();
        p.children()
            .add(link("Source code  ↗", move || host("source", &source)).upcast::<Control>());
    }
}
#[wasm_bindgen(js_name = runMain)]
pub fn run_main(catalog: &str, width: f64) {
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        s.apps = serde_json::from_str(catalog).unwrap_or_default();
        s.width = width;
        s.category = "All apps".into();
    });
    AppBuilder::configure::<App>()
        .with_inter_font()
        .with(Rc::new(FontManagerOptions {
            default_family_name: Some("fonts:Inter#Inter".into()),
            ..Default::default()
        }))
        .start_browser_app(
            "out",
            Some(BrowserPlatformOptions {
                accessibility: BrowserAccessibilityMode::Enabled,
                ..Default::default()
            }),
        );
}
#[wasm_bindgen(js_name = updateCatalog)]
pub fn update_catalog(json: &str) {
    if let Ok(apps) = serde_json::from_str::<Vec<AppInfo>>(json) {
        let changed = STORE.with(|s| {
            let mut s = s.borrow_mut();
            if s.apps == apps {
                false
            } else {
                s.apps = apps;
                true
            }
        });
        if changed {
            render();
        }
    }
}
#[wasm_bindgen]
pub fn navigate(id: &str) {
    STORE.with(|s| s.borrow_mut().selected = if id.is_empty() { None } else { Some(id.into()) });
    render();
    let scroll = STORE.with(|s| s.borrow().scroll.clone());
    if let Some(scroll) = scroll {
        scroll.set_offset(ferroui_base::Vector::new(0.0, 0.0));
    }
}
#[wasm_bindgen]
pub fn resize(width: f64) {
    let old = STORE.with(|s| {
        let mut s = s.borrow_mut();
        let old = s.width;
        s.width = width;
        old
    });
    update_shell(width);
    if (old - width).abs() > 1.0 {
        render();
    }
}
#[wasm_bindgen(js_name = setImage)]
pub fn set_image(url: &str, bytes: &[u8]) {
    if let Ok(bitmap) = Bitmap::from_stream(&mut Cursor::new(bytes)) {
        let bitmap = Rc::new(bitmap);
        STORE.with(|s| {
            let mut s = s.borrow_mut();
            s.images.insert(url.into(), bitmap.clone());
            for (key, i) in &s.waiting {
                if key == url {
                    if let Some(i) = i.upgrade() {
                        i.set_source(Some(bitmap.clone()));
                    }
                }
            }
        });
    }
}
#[wasm_bindgen(js_name = setStatus)]
pub fn set_status(value: &str) {
    STORE.with(|s| {
        let mut s = s.borrow_mut();
        s.last_status = value.into();
        if let Some(t) = &s.status {
            t.set_text(Some(value));
        }
    });
}
#[wasm_bindgen(js_name = uiState)]
pub fn ui_state() -> String {
    STORE.with(|s|{let s=s.borrow();let root=s.grid.as_ref().unwrap().clone().upcast::<Visual>();let bounds=|c:&Control|{let p=c.translate_point(ferroui_base::Point::new(0.0,0.0),&root).unwrap_or_default();let b=c.bounds();serde_json::json!({"x":p.x,"y":p.y,"width":b.width,"height":b.height,"visible":c.is_effectively_visible()})};serde_json::json!({"apps":s.apps.len(),"query":s.query,"category":s.category,"selected":s.selected,"collection":s.collection,"images":s.images.len(),"width":s.width,"buttons":s.controls.iter().map(|(text,b)|{let mut v=bounds(b);v["text"]=serde_json::json!(text);v}).collect::<Vec<_>>(),"searches":s.searches.iter().map(|t|bounds(t)).collect::<Vec<_>>()}).to_string()})
}
#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!("Build with npm run build; serve with npm run preview");
}
