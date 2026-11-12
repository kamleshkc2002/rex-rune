# Rex Linux screenshot inventory

50 visually reviewed, native-resolution PNG captures of Rex Linux 0.6.0
(About: commit `ad196ff`, build date `2026-11-10 PST`, server API revision 13).
Captured on November 11, 2026, Pacific Standard Time, using Cua-driver on
Omarchy/Hyprland. Each original is **2560 × 1410**.

Open **[index.html](index.html)** for the image gallery. Click any thumbnail
to view the original. `manifest.json` records filenames, dimensions, capture
timestamps in Pacific Time, and SHA-256 checksums.

## Coverage

- **00, 30:** terminal workspace and expanded session sidebar.
- **01–08:** all eight Settings sections: Interface, Terminal, Notifications,
  Integrations, Remote Hosts, Shortcuts, Updates, About.
- **09–14:** interface theme picker, terminal font/fallback form, terminal theme
  picker and search, default shell form, lower Terminal settings.
- **15–17:** lower Integrations settings, individual and all-agent setup previews.
- **18–24:** client label, edit/remove host, tailnet discovery, manual endpoint,
  SSH host form, discovered listener details.
- **06, 25–29:** overlapping scroll positions covering the entire Shortcuts section.
- **31–40:** host/session/tab/pane context menus and rename/close dialogs.
- **41–45, 49:** Commands and filtered results, session switcher and search,
  new-session host chooser, inline session rename.
- **46–48:** terminal search, matching results, and no-matches state.

## Scope and final state

This is a visual inventory of the surfaces reachable from the running workspace,
not a functional execution of every action. Creation, split, reconnect, close,
install, and remote-control actions are represented by their menus or previews.
Forms and destructive confirmations were canceled. No session, tab, pane, or
host was created, renamed, removed, or closed; no integration was installed.

Unavailable/context-dependent states are not represented: first-run/empty
workspace, connection and authentication failures, personal SSH-server startup
confirmation, update-available/install progress, already-installed integration
removal, split/zoom layouts, incoming activity notices, or alternate theme variants.
About links open external pages and are represented by the About screen.

The original `still-atoll` session on `shuvbot-rex` remains selected. The sidebar
was restored to hidden, and panels/search were closed. One switcher-search attempt
put five characters into the empty terminal prompt; all five were removed with
Backspace, and no command was submitted. The restored prompt was visually checked.
Omarchy Stay Awake was enabled at the user's request.

`navigation/` contains working captures and review sheets and is excluded from
the deliverable ZIP. The older `/tmp/shuvcode/rex-screenshots` attempt is not part
of this inventory.
