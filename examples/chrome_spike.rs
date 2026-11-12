//! Spike: a borderless Rune window's own minimize / maximize / close, edge resize, drag and
//! window menu, through the vendored Rune additions. `--auto` drives maximize, restore and
//! minimize from a timer and logs what the window reports, for compositors without input.
use rune::{AppSpec, Cursor, Cx, El, Tx, WindowEdge, col, el, row, text};
use std::time::Duration;

fn c(hex: &str) -> rune::Color {
    rune::color::hex(hex).expect("colour")
}

#[derive(Default)]
struct Spike {
    auto: bool,
    step: u32,
    size: (f32, f32),
    maximized: bool,
}

fn button(key: &'static str, label: String, f: impl Fn(&mut Spike, &mut Tx) + 'static) -> El {
    row()
        .key(key)
        .role("button")
        .label(label.clone())
        .h(28.0)
        .px(12.0)
        .center()
        .radius(4.0)
        .fill(c("#182938"))
        .hover(c("#253c50"))
        .on_press_tx(f)
        .on_layout(move |_: &mut Spike, r| {
            eprintln!("spike: rect {key} {} {} {} {}", r[0], r[1], r[2], r[3])
        })
        .child(text(label).color(c("#e3ebf3")))
}

#[allow(clippy::too_many_arguments)]
fn edge(
    key: &'static str,
    e: WindowEdge,
    l: Option<f32>,
    t: Option<f32>,
    r: Option<f32>,
    b: Option<f32>,
    w: Option<f32>,
    h: Option<f32>,
) -> El {
    let mut x = el()
        .key(key)
        .pin(l, t, r, b)
        .window_resize(e)
        .cursor(Cursor::ResizeEdge(e));
    if let Some(w) = w {
        x = x.w(w);
    }
    if let Some(h) = h {
        x = x.h(h);
    }
    x
}

fn view(s: &Spike, cx: &mut Cx<Spike>) -> El {
    let maximized = cx.window_maximized();
    let bar = row()
        .key("bar")
        .h(44.0)
        .w_full()
        .px(8.0)
        .gap(8.0)
        .center_y()
        .fill(c("#091827"))
        .child(
            el().key("drag")
                .grow(1.0)
                .h_full()
                .window_drag()
                .on_secondary_press(|_: &mut Spike, p, tx| tx.show_window_menu(p.x, p.y))
                .child(
                    text("drag · double-click maximizes · right-click menu").color(c("#a0b2c3")),
                ),
        )
        .child(button("min", "Minimize".into(), |_, tx| tx.minimize()))
        .child(button(
            "max",
            if maximized {
                "Restore".into()
            } else {
                "Maximize".into()
            },
            |_, tx| tx.toggle_maximized(),
        ))
        .child(button("close", "Close".into(), |_, tx| tx.close_window()));
    let body = col()
        .key("body")
        .grow(1.0)
        .w_full()
        .center()
        .fill(c("#071522"))
        .on_layout(move |s: &mut Spike, r| {
            if (r[2], r[3]) != s.size || maximized != s.maximized {
                s.size = (r[2], r[3]);
                s.maximized = maximized;
                eprintln!("spike: body {}x{} maximized {maximized}", r[2], r[3]);
            }
        })
        .child(
            text(format!(
                "{:.0}×{:.0} · maximized {maximized} · step {}",
                s.size.0, s.size.1, s.step
            ))
            .color(c("#e3ebf3")),
        );
    let b = 6.0;
    col()
        .key("root")
        .w_full()
        .h_full()
        .child(bar)
        .child(body)
        .child(edge(
            "n",
            WindowEdge::North,
            Some(b),
            Some(0.0),
            Some(b),
            None,
            None,
            Some(b),
        ))
        .child(edge(
            "s",
            WindowEdge::South,
            Some(b),
            None,
            Some(b),
            Some(0.0),
            None,
            Some(b),
        ))
        .child(edge(
            "w",
            WindowEdge::West,
            Some(0.0),
            Some(b),
            None,
            Some(b),
            Some(b),
            None,
        ))
        .child(edge(
            "e",
            WindowEdge::East,
            None,
            Some(b),
            Some(0.0),
            Some(b),
            Some(b),
            None,
        ))
        .child(edge(
            "nw",
            WindowEdge::NorthWest,
            Some(0.0),
            Some(0.0),
            None,
            None,
            Some(b),
            Some(b),
        ))
        .child(edge(
            "ne",
            WindowEdge::NorthEast,
            None,
            Some(0.0),
            Some(0.0),
            None,
            Some(b),
            Some(b),
        ))
        .child(edge(
            "sw",
            WindowEdge::SouthWest,
            Some(0.0),
            None,
            None,
            Some(0.0),
            Some(b),
            Some(b),
        ))
        .child(edge(
            "se",
            WindowEdge::SouthEast,
            None,
            None,
            Some(0.0),
            Some(0.0),
            Some(b),
            Some(b),
        ))
}

fn poll(s: &mut Spike, _: f64, tx: &mut Tx) -> bool {
    if !s.auto {
        return false;
    }
    match s.step {
        0 => {}
        1 => {
            eprintln!("spike: toggle_maximize");
            tx.toggle_maximized();
        }
        2 | 3 => eprintln!(
            "spike: after maximize body {}x{} maximized {}",
            s.size.0, s.size.1, s.maximized
        ),
        4 => {
            eprintln!("spike: toggle_maximize (restore)");
            tx.toggle_maximized();
        }
        5 | 6 => eprintln!(
            "spike: after restore body {}x{} maximized {}",
            s.size.0, s.size.1, s.maximized
        ),
        7 => {
            eprintln!("spike: minimize");
            tx.minimize();
        }
        8 => eprintln!("spike: after minimize (poll still runs)"),
        _ => {
            eprintln!("spike: done");
            tx.quit();
        }
    }
    s.step += 1;
    tx.wake_after(Duration::from_millis(1500));
    true
}

fn main() {
    let auto = std::env::args().any(|a| a == "--auto");
    rune::set_app_id("dev.shuv.rex-rune.spike");
    rune::App::new(AppSpec {
        poll: Some(poll),
        ..AppSpec::new("Rex chrome spike", (800.0, 500.0), Spike::default, view)
    })
    .init(move || Spike {
        auto,
        ..Default::default()
    })
    .args(Vec::new())
    .window_options(rune::WindowOptions {
        borderless: true,
        ..Default::default()
    })
    .run();
}
