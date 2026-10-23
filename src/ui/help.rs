use super::*;

pub(crate) fn render_help_modal(frame: &mut Frame, area: Rect, app: &App) {
    let popup_area = centered_rect(88, 92, area);
    frame.render_widget(Clear, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs bar
            Constraint::Min(10),   // Content
            Constraint::Length(1), // Footer help navigation
        ])
        .split(popup_area);

    let tab_titles = [
        "1:Overview",
        "2:Motions",
        "3:Review",
        "4:Staging",
        "5:Folding",
        "6:Guards",
        "7:CI/Ex",
    ];

    let tab_spans: Vec<Line> = tab_titles
        .iter()
        .enumerate()
        .map(|(idx, title)| {
            if idx == app.help_selected_tab {
                Line::from(vec![Span::styled(
                    format!(" ▶ {} ", title),
                    Style::default()
                        .bg(Color::Cyan)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )])
            } else {
                Line::from(vec![Span::styled(
                    format!(" {} ", title),
                    Style::default().fg(Color::Gray),
                )])
            }
        })
        .collect();

    let guide_title = format!(
        " 📖 LGTM Documentation [Tab {}/7: {}] ",
        app.help_selected_tab + 1,
        tab_titles.get(app.help_selected_tab).unwrap_or(&"")
    );

    let tabs = Tabs::new(tab_spans)
        .select(app.help_selected_tab)
        .divider("│")
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(guide_title)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, chunks[0]);

    let (content_title, help_lines) = match app.help_selected_tab {
        0 => (
            " Tab 1: Overview, Workflow & System Modes ",
            vec![
                Line::from(vec![Span::styled(
                    "HOW TO USE LGTM (CORE WORKFLOW)",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  1. Navigate Diffs: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("Browse changed files and hunks with Vim motions (j/k, ]f/[f, ]h/[h)."),
                ]),
                Line::from(vec![
                    Span::styled("  2. Guardrail Scan: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("Jump between security alerts and complexity spikes with ]w / [w."),
                ]),
                Line::from(vec![
                    Span::styled("  3. Steer Agents:   ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("Add inline directives (c), thread replies (r), blast to tmux (Enter / Ctrl-s)."),
                ]),
                Line::from(vec![
                    Span::styled("  4. Stage & Clean:  ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("Stage good hunks (s), fold clean hunks (zc), discard bad edits (x / X)."),
                ]),
                Line::from(vec![
                    Span::styled("  5. Commit Forge:   ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::raw("Press C to auto-synthesize conventional commits from staged diffs."),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "SYSTEM MODES & KEYBOARD STATES",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  NORMAL Mode        ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Diff navigation, staging (s/u), hunk folding (zc/zo), file review marks (v)"),
                ]),
                Line::from(vec![
                    Span::styled("  COMMAND / EX Mode  ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Press ':' for wildmenu auto-complete (:w, :q, :commit, :prs, :pane, :risk, :<line>)"),
                ]),
                Line::from(vec![
                    Span::styled("  SEARCH Mode        ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Press '/' for regex diff search; cycle matches with 'n' (next) and 'N' (previous)"),
                ]),
                Line::from(vec![
                    Span::styled("  COMMENT Mode       ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Press 'c' to enter inline directive text; press Enter to save or Esc to cancel"),
                ]),
                Line::from(vec![
                    Span::styled("  COMMIT FORGE Mode  ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Press 'C' to edit synthesized conventional commit message and commit to repo"),
                ]),
                Line::from(vec![
                    Span::styled("  PR REVIEW Mode     ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Review open or closed GitHub/Azure PRs with 'lgtm pr <id>' or ':pr <id>'"),
                ]),
                Line::from(vec![
                    Span::styled("  CODEBASE AUDIT Mode", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Run 'lgtm scan repo', 'lgtm audit', or ':scan' to inspect all repository security/arch issues"),
                ]),
                Line::from(vec![
                    Span::styled("  HELP Mode          ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Press '?' or 'h' to open this interactive guide; switch tabs with Tab or 1-7"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "INSTALLATION & DOCUMENTATION",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  Homebrew Install:  ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("brew install mgabs/tap/lgtm"),
                ]),
                Line::from(vec![
                    Span::styled("  GitHub Repo:       ", Style::default().fg(Color::Cyan)),
                    Span::raw("https://github.com/mgabs/lgtm"),
                ]),
                Line::from(vec![
                    Span::styled("  Documentation:     ", Style::default().fg(Color::Cyan)),
                    Span::raw("https://github.com/mgabs/lgtm#readme / docs/WIKI.md"),
                ]),
                Line::from(vec![
                    Span::styled("  Issues & Requests: ", Style::default().fg(Color::Cyan)),
                    Span::raw("https://github.com/mgabs/lgtm/issues"),
                ]),
                Line::from(vec![
                    Span::styled("  Discussions:       ", Style::default().fg(Color::Cyan)),
                    Span::raw("https://github.com/mgabs/lgtm/discussions"),
                ]),
            ],
        ),
        1 => (
            " Tab 2: Vim Motions & Code Navigation ",
            vec![
                Line::from(vec![Span::styled(
                    "CURSOR & SCROLLING",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  j / k, ↓ / ↑     ", Style::default().fg(Color::Cyan)),
                    Span::raw("Move cursor line-by-line through unified/split diff"),
                ]),
                Line::from(vec![
                    Span::styled("  gg / G           ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump cursor to top / bottom of current file diff"),
                ]),
                Line::from(vec![
                    Span::styled("  Ctrl-d / Ctrl-u  ", Style::default().fg(Color::Cyan)),
                    Span::raw("Scroll half-page down / up (15 lines)"),
                ]),
                Line::from(vec![
                    Span::styled("  Ctrl-f / Ctrl-b  ", Style::default().fg(Color::Cyan)),
                    Span::raw("Scroll full page down / up (25 lines) or PgDn / PgUp"),
                ]),
                Line::from(vec![
                    Span::styled("  } / {            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to next / previous paragraph / blank line boundary"),
                ]),
                Line::from(vec![
                    Span::styled("  0 / $            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to first line / last line in current file diff"),
                ]),
                Line::from(vec![
                    Span::styled("  :<number>        ", Style::default().fg(Color::Cyan)),
                    Span::raw("Direct jump to target line number (e.g. :42 or :150)"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "FILE & HUNK BRACKET JUMPS",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  ]f / [f (or ]/[) ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to Next / Previous changed file (press ] or [ to chain)"),
                ]),
                Line::from(vec![
                    Span::styled("  ]h / [h          ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to Next / Previous diff hunk"),
                ]),
                Line::from(vec![
                    Span::styled("  ]c / [c          ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to Next / Previous review directive / inline comment"),
                ]),
                Line::from(vec![
                    Span::styled("  ]t / [t          ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to Next / Previous remote PR thread (after :threads)"),
                ]),
                Line::from(vec![
                    Span::styled("  t / <Space>t     ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open remote PR thread inspector (view full conversation & comments)"),
                ]),
                Line::from(vec![
                    Span::styled("  ]w / [w (or w/W) ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to Next / Previous security alert or guard warning"),
                ]),
                Line::from(vec![
                    Span::styled("  ]u / [u          ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to Next / Previous unreviewed file"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "VIEWPORT & SEARCH CONTROLS",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  /                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Interactive regex search across entire diff"),
                ]),
                Line::from(vec![
                    Span::styled("  n / N            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Jump to next / previous search match"),
                ]),
                Line::from(vec![
                    Span::styled("  * / #            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Search word under cursor forward / backward"),
                ]),
                Line::from(vec![
                    Span::styled("  zz / zt / zb     ", Style::default().fg(Color::Cyan)),
                    Span::raw("Center view on cursor / Cursor to top of view / Cursor to bottom"),
                ]),
                Line::from(vec![
                    Span::styled("  f / Ctrl-p       ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open fuzzy interactive switcher to find and jump to any changed file"),
                ]),
                Line::from(vec![
                    Span::styled("  Tab              ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle file tree drawer sidebar (left panel)"),
                ]),
                Line::from(vec![
                    Span::styled("  d                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle Unified vs Side-by-Side Split Diff view"),
                ]),
                Line::from(vec![
                    Span::styled("  o / e            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open current file:line in editor (default: nvim, config: :editor <name>)"),
                ]),
            ],
        ),
        2 => (
            " Tab 3: Review Directives & Agent Steering ",
            vec![
                Line::from(vec![Span::styled(
                    "INLINE REVIEW DIRECTIVES & AGENT STEERING",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  c (or <Space>c)  ", Style::default().fg(Color::Cyan)),
                    Span::raw("Anchor an inline directive comment directly to the current file:line"),
                ]),
                Line::from(vec![
                    Span::styled("  r                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Thread a reply under directive at cursor (or open new directive)"),
                ]),
                Line::from(vec![
                    Span::styled("  R (or <Space>r)  ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle directive resolved state (✓ RESOLVED vs 💬 OPEN)"),
                ]),
                Line::from(vec![
                    Span::styled("  <Space>d         ", Style::default().fg(Color::Cyan)),
                    Span::raw("Delete the directive anchored on the current line (:del)"),
                ]),
                Line::from(vec![
                    Span::styled("  x                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Discard hunk under cursor (or delete directive if on comment line)"),
                ]),
                Line::from(vec![
                    Span::styled("  <Space>l / :c    ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open interactive Review Directives list drawer (:directives)"),
                ]),
                Line::from(vec![
                    Span::styled("  Ctrl-s / :review ", Style::default().fg(Color::Cyan)),
                    Span::raw("Compile directives to batch prompt and blast to tmux agent / PR / clipboard"),
                ]),
                Line::from(vec![
                    Span::styled("  Enter            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Inject #ref file:line [rule] alert | DIRECTIVE into target agent pane"),
                ]),
                Line::from(vec![
                    Span::styled("  X (or :reject)   ", Style::default().fg(Color::Cyan)),
                    Span::raw("Reject & revert hunk under cursor and re-prompt agent with reason"),
                ]),
                Line::from(vec![
                    Span::styled("  A (or :approve)  ", Style::default().fg(Color::Cyan)),
                    Span::raw("Approve waiting agent breakpoint gate (.lgtm/gate.json)"),
                ]),
                Line::from(vec![
                    Span::styled("  P (or :prompt)   ", Style::default().fg(Color::Cyan)),
                    Span::raw("Inspect agent prompt / gate context modal next to diff"),
                ]),
                Line::from(vec![
                    Span::styled("  T                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Summarize test-risk alerts already present in the current review"),
                ]),
                Line::from(vec![
                    Span::styled("  p (or <Space>p)  ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open pane selector for terminal multiplexer (tmux, zellij, wezterm, kitty)"),
                ]),
                Line::from(vec![
                    Span::styled("  <Space>y / yr    ", Style::default().fg(Color::Cyan)),
                    Span::raw("Yank #ref file:line anchor to system clipboard via OSC 52"),
                ]),
                Line::from(vec![
                    Span::styled("  y                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Yank raw current line content to clipboard"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "REVIEW STATUS MARKS",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  v                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle reviewed status checkmark (✓) for current file (persisted in .lgtm/state.json)"),
                ]),
            ],
        ),
        3 => (
            " Tab 4: Git Staging & Interactive Commit Forge ",
            vec![
                Line::from(vec![Span::styled(
                    "GIT HUNK & FILE STAGING",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  s                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Stage current hunk (or selected visual lines) into Git index"),
                ]),
                Line::from(vec![
                    Span::styled("  S                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Stage entire current file changes into Git index"),
                ]),
                Line::from(vec![
                    Span::styled("  u                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Unstage current hunk (or selected visual lines) from Git index"),
                ]),
                Line::from(vec![
                    Span::styled("  U                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Unstage entire current file from Git index"),
                ]),
                Line::from(vec![
                    Span::styled("  V                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle Visual Line Selection mode to stage/discard arbitrary line ranges"),
                ]),
                Line::from(vec![
                    Span::styled("  <Space>x         ", Style::default().fg(Color::Cyan)),
                    Span::raw("Discard changes in current hunk"),
                ]),
                Line::from(vec![
                    Span::styled("  X                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Discard all working tree changes in current file (git restore)"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "COMMIT FORGE (:commit / C)",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  C / :commit      ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open interactive Commit Forge modal for staged hunks"),
                ]),
                Line::from("  • Automatically synthesizes Conventional Commit messages (feat:, fix:, chore:, refactor:)."),
                Line::from("  • Live edit commit title & body before committing."),
                Line::from("  • Press Enter in Commit Forge to execute git commit immediately with instant feedback."),
            ],
        ),
        4 => (
            " Tab 5: Syntax-Aware AST & Hunk Folding ",
            vec![
                Line::from(vec![Span::styled(
                    "HUNK FOLDING CONTROLS",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  zc               ", Style::default().fg(Color::Cyan)),
                    Span::raw("Fold current hunk into compact header (▶ @@ ... @@ [Folded])"),
                ]),
                Line::from(vec![
                    Span::styled("  zo               ", Style::default().fg(Color::Cyan)),
                    Span::raw("Unfold current hunk to expand and inspect all diff lines"),
                ]),
                Line::from(vec![
                    Span::styled("  za               ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle fold state of current hunk under cursor"),
                ]),
                Line::from(vec![
                    Span::styled("  zM / zm          ", Style::default().fg(Color::Cyan)),
                    Span::raw("Fold ALL hunks across the current file / diff"),
                ]),
                Line::from(vec![
                    Span::styled("  zR / zr          ", Style::default().fg(Color::Cyan)),
                    Span::raw("Unfold ALL hunks across the current file / diff"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "WHY FOLDING MATTERS FOR AGENT REVIEW",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from("  • AI Coding Agents often touch massive boilerplate files alongside critical logic."),
                Line::from("  • Folding unchanged or verified hunks lets you isolate and focus on suspicious diffs."),
                Line::from("  • Line numbers and cursor indexing remain 100% accurate while folded."),
                Line::from("  • Press `zo` on any folded hunk to inspect or stage individual lines."),
            ],
        ),
        5 => (
            " Tab 6: Security Guardrails & Static Analysis ",
            vec![
                Line::from(vec![Span::styled(
                    "REAL-TIME STATIC ANALYSIS & CODE QUALITY GUARDS",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  🔴 Shannon Entropy Secrets: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::raw("Sniffs high-entropy strings (H ≥ 4.3), API keys (sk-proj, ghp), RSA private keys"),
                ]),
                Line::from(vec![
                    Span::styled("  🔴 Injections & Traversal:   ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::raw("Flags os.system, exec, raw SQL string templates, SSRF sinks, and path traversal (..)"),
                ]),
                Line::from(vec![
                    Span::styled("  🔴 Insecure Deserialization: ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
                    Span::raw("Flags unsafe pickle.loads, yaml.load without SafeLoader, and hardcoded auth fallbacks"),
                ]),
                Line::from(vec![
                    Span::styled("  🟠 Cognitive Complexity:    ", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                    Span::raw("Warns when agent code spikes complexity (+5 points or nested loops/conditions)"),
                ]),
                Line::from(vec![
                    Span::styled("  🟠 Identifier Typo Sniffer: ", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                    Span::raw("Levenshtein distance sniffer catching 1-2 char typos against symbol index (e.g. usertoken)"),
                ]),
                Line::from(vec![
                    Span::styled("  🟠 Breaking Public API Drift: ", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                    Span::raw("Detects modified parameter lists or return types on pub fn / export functions"),
                ]),
                Line::from(vec![
                    Span::styled("  🟡 Dead Code Additions (YAGNI): ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw("Flags unreferenced helper functions added by agents with 0 call sites across repo"),
                ]),
                Line::from(vec![
                    Span::styled("  🟠 Test Neutering & Skips:   ", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                    Span::raw("Detects deleted assertions, @pytest.mark.skip, t.Skip, and tautological tests (assert True)"),
                ]),
                Line::from(vec![
                    Span::styled("  🟠 Architecture Sprawl & Leaks: ", Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)),
                    Span::raw("Catches UI->DB layer violations, blast radius sprawl (≥4 domains), and .env file leaks"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "AUDIT & FINDINGS ACTIONS",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  c                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Add review directive / comment anchored to the selected finding"),
                ]),
                Line::from(vec![
                    Span::styled("  Enter            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Inject remediation directive directly to agent pane / clipboard"),
                ]),
                Line::from(vec![
                    Span::styled("  y                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Yank remediation directive to clipboard (OSC 52)"),
                ]),
                Line::from(vec![
                    Span::styled("  e / o            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open file at finding location in $EDITOR"),
                ]),
            ],
        ),
        6 => (
            " Tab 7: Ex-Commands & Headless CI Integration ",
            vec![
                Line::from(vec![Span::styled(
                    "EX-COMMAND REFERENCE (Type ':' in diff view, press Tab to cycle)",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  :w, :write           ", Style::default().fg(Color::Cyan)),
                    Span::raw("Save active directives to .lgtm/comments.jsonl"),
                ]),
                Line::from(vec![
                    Span::styled("  :q, :quit, :exit     ", Style::default().fg(Color::Cyan)),
                    Span::raw("Quit lgtm (:q! force quit)"),
                ]),
                Line::from(vec![
                    Span::styled("  :wq, :x              ", Style::default().fg(Color::Cyan)),
                    Span::raw("Save directives to disk and exit"),
                ]),
                Line::from(vec![
                    Span::styled("  :commit [msg], :ci   ", Style::default().fg(Color::Cyan)),
                    Span::raw("Commit staged changes with message (or open Commit Forge modal)"),
                ]),
                Line::from(vec![
                    Span::styled("  :prs [open|closed|all]", Style::default().fg(Color::Cyan)),
                    Span::raw("Open PR picker; 1/2/3 select open/closed/all"),
                ]),
                Line::from(vec![
                    Span::styled("  :pr <number>         ", Style::default().fg(Color::Cyan)),
                    Span::raw("Load PR in background; close drafts and visual selections to switch"),
                ]),
                Line::from(vec![
                    Span::styled("  :pane [id]           ", Style::default().fg(Color::Cyan)),
                    Span::raw("Select or set target agent pane ID for active multiplexer"),
                ]),
                Line::from(vec![
                    Span::styled("  :sink [name]         ", Style::default().fg(Color::Cyan)),
                    Span::raw("View or set active agent sink (tmux, zellij, wezterm, kitty, clipboard, auto)"),
                ]),
                Line::from(vec![
                    Span::styled("  :c, :directives      ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open review directives drawer (:directives recheck | orphans)"),
                ]),
                Line::from(vec![
                    Span::styled("  :clear / :clear-resolved", Style::default().fg(Color::Cyan)),
                    Span::raw("Clear all directives, or clear only resolved ones"),
                ]),
                Line::from(vec![
                    Span::styled("  :resolve / :unresolve", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle directive resolved state (or :resolve all / :unresolve all)"),
                ]),
                Line::from(vec![
                    Span::styled("  :f, :files, :find    ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open fuzzy file switcher modal"),
                ]),
                Line::from(vec![
                    Span::styled("  :sp, :split, :vsp    ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle Unified vs Side-by-Side Split Diff mode"),
                ]),
                Line::from(vec![
                    Span::styled("  :audit, :scan repo   ", Style::default().fg(Color::Cyan)),
                    Span::raw("Audit the whole codebase (issues only, no diff view); plain :scan covers changes only"),
                ]),
                Line::from(vec![
                    Span::styled("  :export [path]       ", Style::default().fg(Color::Cyan)),
                    Span::raw("Export audit scorecard report to Markdown (default: lgtm-report.md)"),
                ]),
                Line::from(vec![
                    Span::styled("  :w [path]            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Export audit report (in Audit mode) or save review directives (in Diff mode)"),
                ]),
                Line::from(vec![
                    Span::styled("  :diff                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Return to Diff review mode from Audit mode (or specify revision range)"),
                ]),
                Line::from(vec![
                    Span::styled("  :approve / :lgtm     ", Style::default().fg(Color::Cyan)),
                    Span::raw("Approve pending agent breakpoint gate"),
                ]),
                Line::from(vec![
                    Span::styled("  :reject              ", Style::default().fg(Color::Cyan)),
                    Span::raw("Revert hunk under cursor and dispatch prompt with rejection reason"),
                ]),
                Line::from(vec![
                    Span::styled("  :reject-all          ", Style::default().fg(Color::Cyan)),
                    Span::raw("Reject waiting agent breakpoint gate"),
                ]),
                Line::from(vec![
                    Span::styled("  :prompt / :context   ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open agent prompt / gate context modal"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "AUDIT MODE CONTROLS (Active in Codebase Audit view)",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  Tab / h / l          ", Style::default().fg(Color::Cyan)),
                    Span::raw("Cycle active pane focus (Files <-> Issues <-> Inspector)"),
                ]),
                Line::from(vec![
                    Span::styled("  w / W / n / N / ]c / [c", Style::default().fg(Color::Cyan)),
                    Span::raw("Next / Previous issue in list"),
                ]),
                Line::from(vec![
                    Span::styled("  ] / [ / ]f / [f      ", Style::default().fg(Color::Cyan)),
                    Span::raw("Next / Previous file in audit drawer"),
                ]),
                Line::from(vec![
                    Span::styled("  Space / Enter (Files)", Style::default().fg(Color::Cyan)),
                    Span::raw("Filter issues to selected file / isolate file"),
                ]),
                Line::from(vec![
                    Span::styled("  Z / m / b            ", Style::default().fg(Color::Cyan)),
                    Span::raw("Toggle maximized Inspector / toggle file sidebar"),
                ]),
                Line::from(vec![
                    Span::styled("  y / Ctrl-s           ", Style::default().fg(Color::Cyan)),
                    Span::raw("Yank directive to clipboard / batch-inject all to agent"),
                ]),
                Line::from(vec![
                    Span::styled("  1 / 2 / 3 / a        ", Style::default().fg(Color::Cyan)),
                    Span::raw("Filter severity: [1] Critical | [2] High+ | [3] All | [a] Reset all"),
                ]),
                Line::from(vec![
                    Span::styled("  Enter                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Inject violation remediation directive directly to agent tmux pane / clipboard"),
                ]),
                Line::from(vec![
                    Span::styled("  e / o                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Open current issue file:line in editor (default: nvim, config: :editor <name>)"),
                ]),
                Line::from(vec![
                    Span::styled("  w / E                ", Style::default().fg(Color::Cyan)),
                    Span::raw("Quick export repository audit report to lgtm-report.md"),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "HEADLESS CI / CD & PRE-COMMIT GATES",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )]),
                Line::from(vec![
                    Span::styled("  lgtm check           ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Headless gate for GitHub Actions & CI. Exits code 1 on Critical/High risks."),
                ]),
                Line::from(vec![
                    Span::styled("  lgtm scan --tui      ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Launch interactive TUI Codebase Audit mode (`lgtm scan repo` / `lgtm audit`)."),
                ]),
                Line::from(vec![
                    Span::styled("  lgtm scan -o rep.md  ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Generates full Markdown audit scorecard (Grade A/B/C/F) & remediation steps."),
                ]),
                Line::from(vec![
                    Span::styled("  lgtm scan -f sarif   ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::raw("Exports standard SARIF JSON for GitHub Security Code Scanning tab upload."),
                ]),
            ],
        ),
        _ => (" Help ", vec![Line::from("No help content")]),
    };

    let content_block = Paragraph::new(help_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(content_title)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(content_block, chunks[1]);

    let footer_text = Line::from(vec![
        Span::styled(" Tabs: ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            "Tab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" / "),
        Span::styled(
            "l",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" / "),
        Span::styled(
            "→",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Next | "),
        Span::styled(
            "BackTab",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" / "),
        Span::styled(
            "h",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" / "),
        Span::styled(
            "←",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Prev | "),
        Span::styled(
            "1-7",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Jump Tab | "),
        Span::styled(
            "Esc / q / ?",
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Close"),
    ]);
    let footer_widget = Paragraph::new(footer_text);
    frame.render_widget(footer_widget, chunks[2]);
}
