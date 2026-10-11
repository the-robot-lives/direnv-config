"use strict";(()=>{var r=class{static diff(p){let o=performance.now()-p;return o<1?`${Math.round(o*1e3)} \u03BCs`:`${Math.round(o)} ms`}static start(){return performance.now()}};var u=performance.now();globalThis.Hologram.pageReachableFunctionDefs=m=>{let{Bitstring:p,ERTS:o,HologramBoxedError:g,HologramInterpreterError:b,Interpreter:e,MemoryStorage:c,Type:t,Utils:x}=m;e.defineElixirFunction("DirenvConfigWeb.MainLayout","__props__",0,"public",[{params:s=>[],guards:[],body:s=>Elixir_Enum["reverse/1"](t.list([]))}]),e.defineElixirFunction("DirenvConfigWeb.MainLayout","template",0,"public",[{params:s=>[],guards:[],body:s=>(globalThis.Hologram.return=t.anonymousFunction(1,[{params:n=>[t.variablePattern("vars_0")],guards:[],body:n=>(e.matchOperator(n.vars.vars_0,t.matchPlaceholder(),n),e.updateVarsToMatchedValues(n),t.list([t.tuple([t.atom("element"),t.bitstring("html"),t.list([t.tuple([t.bitstring("lang"),t.list([t.tuple([t.atom("text"),t.bitstring("en")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("head"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("meta"),t.list([t.tuple([t.bitstring("charset"),t.list([t.tuple([t.atom("text"),t.bitstring("utf-8")])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("meta"),t.list([t.tuple([t.bitstring("name"),t.list([t.tuple([t.atom("text"),t.bitstring("viewport")])])]),t.tuple([t.bitstring("content"),t.list([t.tuple([t.atom("text"),t.bitstring("width=device-width, initial-scale=1")])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("title"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("direnv-config \u2014 YAML in. Env vars out.")])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("meta"),t.list([t.tuple([t.bitstring("name"),t.list([t.tuple([t.atom("text"),t.bitstring("description")])])]),t.tuple([t.bitstring("content"),t.list([t.tuple([t.atom("text"),t.bitstring("direnv-config (dc) replaces sprawling export VAR=value .envrc files with named, layered YAML configs \u2014 deep-merged and flattened into env vars, with versioned history, file-based IPC, and encrypted-at-rest secrets. Rust CLI, MIT licensed.")])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("meta"),t.list([t.tuple([t.bitstring("property"),t.list([t.tuple([t.atom("text"),t.bitstring("og:type")])])]),t.tuple([t.bitstring("content"),t.list([t.tuple([t.atom("text"),t.bitstring("website")])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("meta"),t.list([t.tuple([t.bitstring("property"),t.list([t.tuple([t.atom("text"),t.bitstring("og:title")])])]),t.tuple([t.bitstring("content"),t.list([t.tuple([t.atom("text"),t.bitstring("direnv-config \u2014 YAML in. Env vars out.")])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("meta"),t.list([t.tuple([t.bitstring("property"),t.list([t.tuple([t.atom("text"),t.bitstring("og:description")])])]),t.tuple([t.bitstring("content"),t.list([t.tuple([t.atom("text"),t.bitstring("One-liner .envrc files loading layered YAML: base \u2192 env overlay \u2192 local \u2192 secrets, deep-merged and flattened into env vars. File-based state any process can read or write; encrypted secrets; timestamped history.")])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("meta"),t.list([t.tuple([t.bitstring("property"),t.list([t.tuple([t.atom("text"),t.bitstring("og:url")])])]),t.tuple([t.bitstring("content"),t.list([t.tuple([t.atom("text"),t.bitstring("https://dirconfig.therobotlives.com/")])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("meta"),t.list([t.tuple([t.bitstring("name"),t.list([t.tuple([t.atom("text"),t.bitstring("twitter:card")])])]),t.tuple([t.bitstring("content"),t.list([t.tuple([t.atom("text"),t.bitstring("summary")])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("link"),t.list([t.tuple([t.bitstring("rel"),t.list([t.tuple([t.atom("text"),t.bitstring("icon")])])]),t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("/assets/favicon.svg")])])]),t.tuple([t.bitstring("type"),t.list([t.tuple([t.atom("text"),t.bitstring("image/svg+xml")])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("link"),t.list([t.tuple([t.bitstring("rel"),t.list([t.tuple([t.atom("text"),t.bitstring("preconnect")])])]),t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("https://fonts.googleapis.com")])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("link"),t.list([t.tuple([t.bitstring("rel"),t.list([t.tuple([t.atom("text"),t.bitstring("preconnect")])])]),t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("https://fonts.gstatic.com")])])]),t.tuple([t.bitstring("crossorigin"),t.list([t.tuple([t.atom("text"),t.bitstring("anonymous")])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("link"),t.list([t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500;700&display=swap")])])]),t.tuple([t.bitstring("rel"),t.list([t.tuple([t.atom("text"),t.bitstring("stylesheet")])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("link"),t.list([t.tuple([t.bitstring("rel"),t.list([t.tuple([t.atom("text"),t.bitstring("stylesheet")])])]),t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("/assets/css/site.css")])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("component"),t.atom("Elixir.Hologram.UI.Runtime"),t.list([]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
  `)])])]),t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("body"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("a"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("skip-link")])])]),t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("#main")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("Skip to content")])])]),t.tuple([t.atom("text"),t.bitstring(`

    `)]),t.tuple([t.atom("element"),t.bitstring("header"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("site-header")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("container header-inner")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("a"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("brand")])])]),t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("#top")])])]),t.tuple([t.bitstring("aria-label"),t.list([t.tuple([t.atom("text"),t.bitstring("direnv-config home")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("span"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("brand-mark")])])]),t.tuple([t.bitstring("aria-hidden"),t.list([t.tuple([t.atom("text"),t.bitstring("true")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("dc")])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("span"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("brand-name")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("direnv-config")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("nav"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("site-nav")])])]),t.tuple([t.bitstring("aria-label"),t.list([t.tuple([t.atom("text"),t.bitstring("Primary")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("a"),t.list([t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("#why")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("Why")])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("a"),t.list([t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("#how")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("How it works")])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("a"),t.list([t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("#cli")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("CLI & trust")])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("a"),t.list([t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("#faq")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("FAQ")])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("a"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("btn btn-primary btn-sm")])])]),t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("#install")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("Install")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`

    `)]),t.tuple([t.atom("element"),t.bitstring("main"),t.list([t.tuple([t.bitstring("id"),t.list([t.tuple([t.atom("text"),t.bitstring("main")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("slot"),t.list([]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`

    `)]),t.tuple([t.atom("element"),t.bitstring("footer"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("site-footer")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("container footer-inner")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("footer-brand")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("direnv-config \u2014 a noizu.com project")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("footer-line")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("a"),t.list([t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("https://github.com/the-robot-lives/direnv-config")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("GitHub")])])]),t.tuple([t.atom("text"),t.bitstring(` \xB7
          `)]),t.tuple([t.atom("element"),t.bitstring("a"),t.list([t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("https://github.com/the-robot-lives/direnv-config/blob/develop/LICENSE")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("MIT licensed")])])]),t.tuple([t.atom("text"),t.bitstring(` \xB7
          This site is Elixir Hologram. \xB7 \xA9 2026
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
  `)])])]),t.tuple([t.atom("text"),t.bitstring(`
`)])])])]))}],s),e.updateVarsToMatchedValues(s),globalThis.Hologram.return)}]),e.defineElixirFunction("DirenvConfigWeb.Pages.HomePage","__layout_module__",0,"public",[{params:s=>[],guards:[],body:s=>t.atom("Elixir.DirenvConfigWeb.MainLayout")}]),e.defineElixirFunction("DirenvConfigWeb.Pages.HomePage","__layout_props__",0,"public",[{params:s=>[],guards:[],body:s=>t.list([])}]),e.defineElixirFunction("DirenvConfigWeb.Pages.HomePage","__params__",0,"public",[{params:s=>[],guards:[],body:s=>Elixir_Enum["reverse/1"](t.list([]))}]),e.defineElixirFunction("DirenvConfigWeb.Pages.HomePage","__route__",0,"public",[{params:s=>[],guards:[],body:s=>t.bitstring("/")}]),e.defineElixirFunction("DirenvConfigWeb.Pages.HomePage","template",0,"public",[{params:s=>[],guards:[],body:s=>(globalThis.Hologram.return=t.anonymousFunction(1,[{params:n=>[t.variablePattern("vars_0")],guards:[],body:n=>(e.matchOperator(n.vars.vars_0,t.matchPlaceholder(),n),e.updateVarsToMatchedValues(n),t.list([t.tuple([t.atom("public_comment"),t.list([t.tuple([t.atom("text"),t.bitstring(" HERO ")])])]),t.tuple([t.atom("text"),t.bitstring(`
`)]),t.tuple([t.atom("element"),t.bitstring("section"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("hero")])])]),t.tuple([t.bitstring("id"),t.list([t.tuple([t.atom("text"),t.bitstring("top")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("container hero-inner")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("eyebrow")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("YAML-backed config layer for direnv \xB7 Rust \xB7 MIT")])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("h1"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("YAML in. Env vars out.")])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("lede")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      direnv-config replaces sprawling `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("export VAR=value")])])]),t.tuple([t.atom("text"),t.bitstring(" ")]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(".envrc")])])]),t.tuple([t.atom("text"),t.bitstring(` files
      with named, layered YAML configs \u2014 deep-merged and flattened into environment variables.
      Because state lives in files, any process can read or write it: parent shell, child script,
      sibling tab, a coding agent.
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("cta-row")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("a"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("btn btn-primary btn-lg")])])]),t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("#install")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("Install from source")])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("a"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("btn btn-ghost btn-lg")])])]),t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("#how")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("How it works")])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("cta-fine")])])])]),t.list([t.tuple([t.atom("element"),t.bitstring("em"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Local-first state \xB7 secrets encrypted at rest \xB7 MIT licensed")])])])])]),t.tuple([t.atom("text"),t.bitstring(`

    `)]),t.tuple([t.atom("element"),t.bitstring("figure"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("install-block")])])]),t.tuple([t.bitstring("id"),t.list([t.tuple([t.atom("text"),t.bitstring("install")])])]),t.tuple([t.bitstring("aria-label"),t.list([t.tuple([t.atom("text"),t.bitstring("Install commands")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("figcaption"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("install-title")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("Install \u2014 clone + make (prerequisites: direnv hooked into your shell, plus a Rust toolchain)")])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("pre"),t.list([]),t.list([t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`git clone https://github.com/the-robot-lives/direnv-config && cd direnv-config

make compile            # cargo build --release
make test
make install            # dc binary + direnv stdlib + shell hook
make install-completions
make doctor             # verify the installation`)])])])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("install-note")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      dc is not on crates.io or Homebrew yet \u2014 it installs from source. Packaged installs are planned.
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`

    `)]),t.tuple([t.atom("element"),t.bitstring("figure"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("install-block")])])]),t.tuple([t.bitstring("aria-label"),t.list([t.tuple([t.atom("text"),t.bitstring("Project .envrc example")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("figcaption"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("install-title")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("Then, in a project .envrc")])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("pre"),t.list([]),t.list([t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`dc_yaml cluster <<'YAML'
name: acme
kubeconfig: ~/.kube/acme/config
YAML
dc_export`)])])])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
  `)])])]),t.tuple([t.atom("text"),t.bitstring(`
`)])])]),t.tuple([t.atom("text"),t.bitstring(`

`)]),t.tuple([t.atom("public_comment"),t.list([t.tuple([t.atom("text"),t.bitstring(" BENEFITS (3-up) ")])])]),t.tuple([t.atom("text"),t.bitstring(`
`)]),t.tuple([t.atom("element"),t.bitstring("section"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("section")])])]),t.tuple([t.bitstring("id"),t.list([t.tuple([t.atom("text"),t.bitstring("features")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("container")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("h2"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("visually-hidden")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("Why direnv-config")])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("benefit-grid")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("article"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("card")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("h3"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("One-liner .envrc, structured YAML")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(".envrc")])])]),t.tuple([t.atom("text"),t.bitstring(` files grow 50+ export lines. direnv-config replaces them with a
          single `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc_yaml")])])]),t.tuple([t.atom("text"),t.bitstring(` heredoc per named config \u2014 or whole YAML files loaded
          with `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc load")])])]),t.tuple([t.atom("text"),t.bitstring(`, several subjects per file. Values are nested keys in
          layered YAML \u2014 `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("base.yaml")])])]),t.tuple([t.atom("text"),t.bitstring(" \u2192 env overlay \u2192 ")]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("local.yaml")])])]),t.tuple([t.atom("text"),t.bitstring(` \u2192
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("secrets.yaml")])])]),t.tuple([t.atom("text"),t.bitstring(` \u2014 deep-merged Elixir-style, then flattened into env vars by
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("_dc")])])]),t.tuple([t.atom("text"),t.bitstring(` rules.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("article"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("card")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("h3"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("File-based state is IPC")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          Env vars are write-once from a parent process's view \u2014 a child can't update them. dc's
          state lives in files, so any process (parent, child, sibling, a coding agent, a
          terminal-tab script) can read or mutate shared config. Every write bumps a monotonic
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(".version")])])]),t.tuple([t.atom("text"),t.bitstring(" counter and snapshots to ")]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("history/")])])]),t.tuple([t.atom("text"),t.bitstring(`; a
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("precmd")])])]),t.tuple([t.atom("text"),t.bitstring(` hook reloads the shell on change.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("article"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("card")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("h3"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Secrets, first-class")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          A separate gitignored `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("secrets.yaml")])])]),t.tuple([t.atom("text"),t.bitstring(`; values encrypted at rest
          (XChaCha20-Poly1305, key in a 0600 lockable file); generated placeholder secrets;
          masked output by default with audited reveal; bidirectional Infisical sync that
          compares digests, never plaintext.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
  `)])])]),t.tuple([t.atom("text"),t.bitstring(`
`)])])]),t.tuple([t.atom("text"),t.bitstring(`

`)]),t.tuple([t.atom("public_comment"),t.list([t.tuple([t.atom("text"),t.bitstring(" WHY TABLE ")])])]),t.tuple([t.atom("text"),t.bitstring(`
`)]),t.tuple([t.atom("element"),t.bitstring("section"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("section section-why")])])]),t.tuple([t.bitstring("id"),t.list([t.tuple([t.atom("text"),t.bitstring("why")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("container")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("h2"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("The .envrc you have vs. the one you could have")])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("section-lede")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      Straight from the README's problem/fix table \u2014 every row is a mechanism, not a mood.
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("table-scroll")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("table"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("why-table")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("thead"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("tr"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("th"),t.list([t.tuple([t.bitstring("scope"),t.list([t.tuple([t.atom("text"),t.bitstring("col")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(".envrc today")])])]),t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("th"),t.list([t.tuple([t.bitstring("scope"),t.list([t.tuple([t.atom("text"),t.bitstring("col")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("direnv-config fix")])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("tbody"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("tr"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(".envrc files grow 50+ export lines")])])]),t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("One-liner .envrc loading structured YAML")])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("tr"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("No hierarchy or grouping")])])]),t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Named configs with nested keys")])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("tr"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Child processes can't update parent config")])])]),t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Edit the YAML; parent reads next access")])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("tr"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("No layering (dev vs prod vs local)")])])]),t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Elixir-style deep merge across layer files")])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("tr"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Secrets mixed with plain config")])])]),t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Separate secrets.yaml (gitignored)")])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("tr"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("No history")])])]),t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Timestamped snapshots on every write")])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("tr"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("No sharing with external state (tab titles, etc.)")])])]),t.tuple([t.atom("text"),t.bitstring(`
            `)]),t.tuple([t.atom("element"),t.bitstring("td"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring('dc_set tab status "deploying" \u2014 file-based IPC')])])]),t.tuple([t.atom("text"),t.bitstring(`
          `)])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
  `)])])]),t.tuple([t.atom("text"),t.bitstring(`
`)])])]),t.tuple([t.atom("text"),t.bitstring(`

`)]),t.tuple([t.atom("public_comment"),t.list([t.tuple([t.atom("text"),t.bitstring(" EXTERNAL FILES (dc load) ")])])]),t.tuple([t.atom("text"),t.bitstring(`
`)]),t.tuple([t.atom("element"),t.bitstring("section"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("section")])])]),t.tuple([t.bitstring("id"),t.list([t.tuple([t.atom("text"),t.bitstring("files")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("container")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("h2"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Config can live in real files, not heredocs")])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("section-lede")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      The `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc_yaml")])])]),t.tuple([t.atom("text"),t.bitstring(` heredoc is the workhorse \u2014 but a config can outgrow it.
      `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc load")])])]),t.tuple([t.atom("text"),t.bitstring(` ingests ordinary YAML files through the exact same path: several
      subjects per file, deep-merged, layer-routed.
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("figure"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("install-block")])])]),t.tuple([t.bitstring("aria-label"),t.list([t.tuple([t.atom("text"),t.bitstring("External YAML file example")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("figcaption"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("install-title")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("team.yaml \u2014 root keys are subject names; each value is that subject's config")])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("pre"),t.list([]),t.list([t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`cluster:
  name: acme
  kubeconfig: ~/.kube/acme/config
ci:
  _dc_layer: local          # per-subject layer routing, stripped before ingest
  runner: linux
cf:
  access:
    client_secret: "\u{1F512} not-a-real-secret"   # \u{1F512} prefix = encrypted at ingest`)])])])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("figure"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("install-block cli-block")])])]),t.tuple([t.bitstring("aria-label"),t.list([t.tuple([t.atom("text"),t.bitstring("dc load commands")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("pre"),t.list([]),t.list([t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`dc load team.yaml shared.yaml   # argument order is load order \u2014 later files win on conflict
dc load --layer dev dev.yaml    # default layer for subjects without _dc_layer
# in .envrc, the shim re-ingests on every direnv source:
dc_load team.yaml shared.yaml
dc_export`)])])])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("section-lede")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      Reads and writes stay provenance-routed. `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc config get")])])]),t.tuple([t.atom("text"),t.bitstring(` prints the owning
      `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("file:line")])])]),t.tuple([t.atom("text"),t.bitstring("; ")]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc config set")])])]),t.tuple([t.atom("text"),t.bitstring(` splices the file the key actually came
      from \u2014 comments preserved \u2014 and refuses to guess when a key matches in more than one
      place, naming every candidate. `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("--file")])])]),t.tuple([t.atom("text"),t.bitstring(` forces the destination; new keys land
      in the last-loaded file.
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("benefit-grid")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("article"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("card")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("h3"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Load order is precedence")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          Files deep-merge in argument order \u2014 later files win on conflict \u2014 and layers
          resolve by their existing weights; there is no "inline beats external" rule. A
          subject name that would collide with a root key goes under a
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("subjects:")])])]),t.tuple([t.atom("text"),t.bitstring(` wrapper; mixing the wrapper with other root keys is an
          error, not a guess.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("article"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("card")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("h3"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Refreshes are faithful")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          Loaded files are tracked \u2014 path, mtime, size, and load options \u2014 in the store's
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(".meta")])])]),t.tuple([t.atom("text"),t.bitstring(` registry, so direnv re-ingests exactly what you loaded. A file
          loaded with `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("--if-missing")])])]),t.tuple([t.atom("text"),t.bitstring(` seeds the store once; its values are
          store-owned afterwards, and refreshes never replay `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("--replace")])])]),t.tuple([t.atom("text"),t.bitstring(`.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("article"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("card")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("h3"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Same secrets, any file")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          Encryption is per-value and location-agnostic: a `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("\u{1F512}")])])]),t.tuple([t.atom("text"),t.bitstring(`-marked value is
          XChaCha20-Poly1305-encrypted at ingest whether it arrives in a heredoc or an
          external file. Moving config into files changes nothing about how secrets are
          stored, masked, or revealed.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
  `)])])]),t.tuple([t.atom("text"),t.bitstring(`
`)])])]),t.tuple([t.atom("text"),t.bitstring(`

`)]),t.tuple([t.atom("public_comment"),t.list([t.tuple([t.atom("text"),t.bitstring(" HOW IT WORKS ")])])]),t.tuple([t.atom("text"),t.bitstring(`
`)]),t.tuple([t.atom("element"),t.bitstring("section"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("section")])])]),t.tuple([t.bitstring("id"),t.list([t.tuple([t.atom("text"),t.bitstring("how")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("container")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("h2"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("How it works")])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("ol"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("steps")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("li"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("h3"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Declare configs in .envrc")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          One `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc_yaml <name> <<'YAML'")])])]),t.tuple([t.atom("text"),t.bitstring(` heredoc per named config \u2014 or a
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc_load <file>\u2026")])])]),t.tuple([t.atom("text"),t.bitstring(` line for external YAML files \u2014 then
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc_export")])])]),t.tuple([t.atom("text"),t.bitstring(`. Stores form an inheritance chain down the directory tree:
          child directories deep-merge their ancestors' configs, ancestor-first.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("li"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("h3"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc resolves layers into state")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          Each config lives under `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("~/.local/state/direnv-config/<path-hash>/")])])]),t.tuple([t.atom("text"),t.bitstring(`
          with its layer files, a resolved `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(".active")])])]),t.tuple([t.atom("text"),t.bitstring(" merge, a ")]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(".version")])])]),t.tuple([t.atom("text"),t.bitstring(`
          counter, and `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("history/")])])]),t.tuple([t.atom("text"),t.bitstring(`. Externally loaded files are registered in
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(".meta")])])]),t.tuple([t.atom("text"),t.bitstring(` so refreshes replay the same ingest. Every mutating call
          regenerates `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(".active")])])]),t.tuple([t.atom("text"),t.bitstring(` and writes a timestamped snapshot;
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("_dc_prune")])])]),t.tuple([t.atom("text"),t.bitstring(` tombstones cut inheritance where you don't want it.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("li"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("h3"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Every shell sees the change")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          A `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("precmd")])])]),t.tuple([t.atom("text"),t.bitstring(" hook watches ")]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(".version")])])]),t.tuple([t.atom("text"),t.bitstring(` and reloads the environment on
          the next prompt. Read-modify-write commands take an advisory `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("flock")])])]),t.tuple([t.atom("text"),t.bitstring(`, so
          concurrent shells can't clobber each other's writes. `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc set")])])]),t.tuple([t.atom("text"),t.bitstring(` from a deploy
          script updates the parent shell's config \u2014 and its terminal tab title.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
  `)])])]),t.tuple([t.atom("text"),t.bitstring(`
`)])])]),t.tuple([t.atom("text"),t.bitstring(`

`)]),t.tuple([t.atom("public_comment"),t.list([t.tuple([t.atom("text"),t.bitstring(" CLI + TRUST ")])])]),t.tuple([t.atom("text"),t.bitstring(`
`)]),t.tuple([t.atom("element"),t.bitstring("section"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("section section-why")])])]),t.tuple([t.bitstring("id"),t.list([t.tuple([t.atom("text"),t.bitstring("cli")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("container")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("h2"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("23 subcommands, one small binary")])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("section-lede")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      Rust, built with clap; zsh and bash completions ship in the box.
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("figure"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("install-block cli-block")])])]),t.tuple([t.bitstring("aria-label"),t.list([t.tuple([t.atom("text"),t.bitstring("dc CLI surface")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("pre"),t.list([]),t.list([t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`dc yaml <name>                  # deep-merge YAML from stdin (--replace, --replace-key, --if-missing, --layer)
dc load <file> [<file>...]     # ingest external YAML files \u2014 load order is precedence
dc get / set / unset / bump     # read values; mutate values and the version counter
dc list / status / env          # configs, merge state, resolved environment
dc gen / gen-secrets            # generate values and placeholder secrets
dc encrypt / decrypt / secrets  # secret material; masked by default, reveal is audited
dc bat                          # browse configs with secrets masked, regex filters
dc infisical / compare / push   # sync against Infisical \u2014 digests, never plaintext
dc keys / config / init         # key file, tool configuration, scaffold a store
dc prune / purge                # hygiene: tombstone inheritance, remove state`)])])])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("benefit-grid")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("article"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("card")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("h3"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("A published threat model")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("docs/THREAT-MODEL.md")])])]),t.tuple([t.atom("text"),t.bitstring(` walks a STRIDE register \u2014 store tampering, key
          exfiltration, log disclosure \u2014 and states the residual risk plainly: dc trusts
          same-user processes; the 0600 key file raises the bar, it is not a security boundary.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("article"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("card")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("h3"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("A published test-health report")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          171 CLI tests plus five SDK suites (Rust, TypeScript, Python, Elixir, PHP) kept
          aligned by shared contract fixtures \u2014 CLI count re-measured 2026-10-12; see
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("docs/TEST-HEALTH.md")])])]),t.tuple([t.atom("text"),t.bitstring(`.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("article"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("card")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("h3"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Honest defaults")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          Secret values are masked in output by default; reveals are written to a 0600 audit
          log; remote sync compares digests, not plaintext; nothing leaves the machine without
          an explicit sync command.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
  `)])])]),t.tuple([t.atom("text"),t.bitstring(`
`)])])]),t.tuple([t.atom("text"),t.bitstring(`

`)]),t.tuple([t.atom("public_comment"),t.list([t.tuple([t.atom("text"),t.bitstring(" FAQ ")])])]),t.tuple([t.atom("text"),t.bitstring(`
`)]),t.tuple([t.atom("element"),t.bitstring("section"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("section section-faq")])])]),t.tuple([t.bitstring("id"),t.list([t.tuple([t.atom("text"),t.bitstring("faq")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("container container-narrow")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("h2"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Questions you'd ask before trusting it")])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("faq-list")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("details"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("faq-item")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("summary"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Is this just .env files with extra steps?")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          No. Named configs give you grouping and nested keys; layers give you dev/prod/local
          overrides without duplication; the parent chain gives you inheritance down the
          directory tree; every write is versioned and snapshotted. Plain exports give you none
          of that.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("details"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("faq-item")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("summary"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Where does my config actually live?")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          In the sources you declare: `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc_yaml")])])]),t.tuple([t.atom("text"),t.bitstring(` heredocs in
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(".envrc")])])]),t.tuple([t.atom("text"),t.bitstring(", plus any YAML files registered with ")]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc load")])])]),t.tuple([t.atom("text"),t.bitstring(` \u2014
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc config get")])])]),t.tuple([t.atom("text"),t.bitstring(" names the exact ")]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("file:line")])])]),t.tuple([t.atom("text"),t.bitstring(` a key came from.
          Resolved state lives under `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("~/.local/state/direnv-config/")])])]),t.tuple([t.atom("text"),t.bitstring(` (one
          directory per project path hash), with the merged `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(".active")])])]),t.tuple([t.atom("text"),t.bitstring(` snapshot,
          the `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(".version")])])]),t.tuple([t.atom("text"),t.bitstring(" counter, and ")]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("history/")])])]),t.tuple([t.atom("text"),t.bitstring(`.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("details"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("faq-item")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("summary"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Can two shells write at the same time?")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          Yes. Every read-modify-write command takes an advisory `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("flock")])])]),t.tuple([t.atom("text"),t.bitstring(` on the
          store, so simultaneous invocations can't lose writes or version bumps. A layer file
          that exists but fails to parse is refused, not silently replaced.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("details"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("faq-item")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("summary"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("Do secrets ever leave my machine?")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          Only when you run an explicit sync command (`)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc infisical")])])]),t.tuple([t.atom("text"),t.bitstring(`,
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc compare")])])]),t.tuple([t.atom("text"),t.bitstring(", ")]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc push")])])]),t.tuple([t.atom("text"),t.bitstring(`). Comparison uses digests \u2014 plaintext
          never crosses the wire \u2014 and the default output path masks secret values everywhere.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("details"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("faq-item")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("summary"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("What if I lose the machine holding the key?")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          Encrypted values are unrecoverable without the key file \u2014 back it up.
          `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("dc keys status | lock | unlock | migrate")])])]),t.tuple([t.atom("text"),t.bitstring(` manages the key file; legacy
          keys embedded in `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("settings.yaml")])])]),t.tuple([t.atom("text"),t.bitstring(` are still read transparently.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("details"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("faq-item")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("summary"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("How do I install it?")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
          Clone the repo and `)]),t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("make install")])])]),t.tuple([t.atom("text"),t.bitstring(` \u2014 see the box at the top. You need
          direnv hooked into your shell and a Rust toolchain. It is not on crates.io or
          Homebrew yet; packaged installs are planned.
        `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("details"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("faq-item")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("summary"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("What's the license?")])])]),t.tuple([t.atom("text"),t.bitstring(`
        `)]),t.tuple([t.atom("element"),t.bitstring("p"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("MIT. Fork it, read the threat model, run the demo.")])])]),t.tuple([t.atom("text"),t.bitstring(`
      `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
  `)])])]),t.tuple([t.atom("text"),t.bitstring(`
`)])])]),t.tuple([t.atom("text"),t.bitstring(`

`)]),t.tuple([t.atom("public_comment"),t.list([t.tuple([t.atom("text"),t.bitstring(" CLOSING CTA ")])])]),t.tuple([t.atom("text"),t.bitstring(`
`)]),t.tuple([t.atom("element"),t.bitstring("section"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("closing")])])]),t.tuple([t.bitstring("id"),t.list([t.tuple([t.atom("text"),t.bitstring("get-started")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("container closing-inner")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("h2"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring("YAML in. Env vars out.")])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("figure"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("install-block")])])]),t.tuple([t.bitstring("aria-label"),t.list([t.tuple([t.atom("text"),t.bitstring("Install commands")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("pre"),t.list([]),t.list([t.tuple([t.atom("element"),t.bitstring("code"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`git clone https://github.com/the-robot-lives/direnv-config && cd direnv-config
make compile && make test
make install && make doctor`)])])])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)]),t.tuple([t.atom("element"),t.bitstring("div"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("cta-row")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring(`
      `)]),t.tuple([t.atom("element"),t.bitstring("a"),t.list([t.tuple([t.bitstring("class"),t.list([t.tuple([t.atom("text"),t.bitstring("btn btn-primary btn-lg")])])]),t.tuple([t.bitstring("href"),t.list([t.tuple([t.atom("text"),t.bitstring("#install")])])])]),t.list([t.tuple([t.atom("text"),t.bitstring("Install from source")])])]),t.tuple([t.atom("text"),t.bitstring(`
    `)])])]),t.tuple([t.atom("text"),t.bitstring(`
  `)])])]),t.tuple([t.atom("text"),t.bitstring(`
`)])])])]))}],s),e.updateVarsToMatchedValues(s),globalThis.Hologram.return)}]),e.defineElixirFunction("Hologram.Router.Helpers","page_bundle_path",1,"public",[{params:s=>[t.variablePattern("page_digest_0")],guards:[],body:s=>t.bitstring([t.bitstringSegment(t.string("/hologram/page-"),{type:"binary"}),t.bitstringSegment(Elixir_String_Chars["to_string/1"](s.vars.page_digest_0),{type:"binary"}),t.bitstringSegment(t.string(".js"),{type:"binary"})])}]),e.defineElixirFunction("Hologram.UI.Runtime","__props__",0,"public",[{params:s=>[],guards:[],body:s=>Elixir_Enum["reverse/1"](t.list([t.tuple([t.atom("page_mounted?"),t.atom("boolean"),t.list([t.tuple([t.atom("from_context"),t.tuple([t.atom("Elixir.Hologram.Runtime"),t.atom("page_mounted?")])])])]),t.tuple([t.atom("page_digest"),t.atom("string"),t.list([t.tuple([t.atom("from_context"),t.tuple([t.atom("Elixir.Hologram.Runtime"),t.atom("page_digest")])])])]),t.tuple([t.atom("instance_id"),t.atom("string"),t.list([t.tuple([t.atom("from_context"),t.tuple([t.atom("Elixir.Hologram.Runtime"),t.atom("instance_id")])])])]),t.tuple([t.atom("initial_page?"),t.atom("boolean"),t.list([t.tuple([t.atom("from_context"),t.tuple([t.atom("Elixir.Hologram.Runtime"),t.atom("initial_page?")])])])]),t.tuple([t.atom("csrf_token"),t.atom("string"),t.list([t.tuple([t.atom("from_context"),t.tuple([t.atom("Elixir.Hologram.Runtime"),t.atom("csrf_token")])])])])]))}]),e.defineElixirFunction("Hologram.UI.Runtime","template",0,"public",[{params:s=>[],guards:[],body:s=>(globalThis.Hologram.return=t.anonymousFunction(1,[{params:n=>[t.variablePattern("vars_0")],guards:[],body:n=>(e.matchOperator(n.vars.vars_0,t.matchPlaceholder(),n),e.updateVarsToMatchedValues(n),t.list([e.case(e.case(e.dotOperator(n.vars.vars_0,t.atom("initial_page?")),[{match:t.variablePattern("x_1"),guards:[i=>Erlang["orelse/2"](l=>Erlang["=:=/2"](l.vars.x_1,t.atom("false")),l=>Erlang["=:=/2"](l.vars.x_1,t.atom("nil")),i)],body:i=>i.vars.x_1},{match:t.matchPlaceholder(),guards:[],body:i=>e.case(e.dotOperator(i.vars.vars_0,t.atom("page_mounted?")),[{match:t.variablePattern("x_2"),guards:[l=>Erlang["orelse/2"](a=>Erlang["=:=/2"](a.vars.x_2,t.atom("false")),a=>Erlang["=:=/2"](a.vars.x_2,t.atom("nil")),l)],body:l=>t.atom("true")},{match:t.matchPlaceholder(),guards:[],body:l=>t.atom("false")}],i)}],n),[{match:t.variablePattern("x_3"),guards:[i=>Erlang["orelse/2"](l=>Erlang["=:=/2"](l.vars.x_3,t.atom("false")),l=>Erlang["=:=/2"](l.vars.x_3,t.atom("nil")),i)],body:i=>t.atom("nil")},{match:t.matchPlaceholder(),guards:[],body:i=>t.list([t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("script"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
    globalThis.Hologram ??= {};
    globalThis.Hologram._pendingJsInteropActions = [];
    globalThis.Hologram.assetManifest = $ASSET_MANIFEST_JS_PLACEHOLDER;
    globalThis.Hologram.csrfToken = "`)]),t.tuple([t.atom("expression"),t.tuple([e.dotOperator(i.vars.vars_0,t.atom("csrf_token"))])]),t.tuple([t.atom("text"),t.bitstring(`";
    globalThis.Hologram.instanceId = "`)]),t.tuple([t.atom("expression"),t.tuple([e.dotOperator(i.vars.vars_0,t.atom("instance_id"))])]),t.tuple([t.atom("text"),t.bitstring(`";

    globalThis.Hologram.dispatchAction = function(actionName, target, params) {
      globalThis.Hologram._pendingJsInteropActions.push([actionName, target, params]);
    }
  `)])])]),t.tuple([t.atom("text"),t.bitstring(`
`)])])}],n),t.tuple([t.atom("text"),t.bitstring(`

`)]),e.case(e.case(e.dotOperator(n.vars.vars_0,t.atom("page_mounted?")),[{match:t.variablePattern("x_4"),guards:[i=>Erlang["orelse/2"](l=>Erlang["=:=/2"](l.vars.x_4,t.atom("false")),l=>Erlang["=:=/2"](l.vars.x_4,t.atom("nil")),i)],body:i=>t.atom("true")},{match:t.matchPlaceholder(),guards:[],body:i=>t.atom("false")}],n),[{match:t.atom("false"),guards:[],body:i=>t.atom("nil")},{match:t.atom("true"),guards:[],body:i=>t.list([t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("script"),t.list([]),t.list([t.tuple([t.atom("text"),t.bitstring(`
    
      globalThis.Hologram.pageMountData = (deps) => {
        const Type = deps.Type;
        
        return {
          componentRegistry: $COMPONENT_REGISTRY_JS_PLACEHOLDER,
          pageModule: $PAGE_MODULE_JS_PLACEHOLDER,
          pageParams: $PAGE_PARAMS_JS_PLACEHOLDER,
          selfEchoes: $SELF_ECHOES_JS_PLACEHOLDER,
          subReceiptAdds: $SUB_RECEIPT_ADDS_JS_PLACEHOLDER,
          subReceiptDrops: $SUB_RECEIPT_DROPS_JS_PLACEHOLDER
        };
      };
    `)]),t.tuple([t.atom("text"),t.bitstring(`
  `)])])]),t.tuple([t.atom("text"),t.bitstring(`
`)])])}],n),t.tuple([t.atom("text"),t.bitstring(`

`)]),e.case(e.case(e.dotOperator(n.vars.vars_0,t.atom("initial_page?")),[{match:t.variablePattern("x_5"),guards:[i=>Erlang["orelse/2"](l=>Erlang["=:=/2"](l.vars.x_5,t.atom("false")),l=>Erlang["=:=/2"](l.vars.x_5,t.atom("nil")),i)],body:i=>i.vars.x_5},{match:t.matchPlaceholder(),guards:[],body:i=>e.case(e.dotOperator(i.vars.vars_0,t.atom("page_mounted?")),[{match:t.variablePattern("x_6"),guards:[l=>Erlang["orelse/2"](a=>Erlang["=:=/2"](a.vars.x_6,t.atom("false")),a=>Erlang["=:=/2"](a.vars.x_6,t.atom("nil")),l)],body:l=>t.atom("true")},{match:t.matchPlaceholder(),guards:[],body:l=>t.atom("false")}],i)}],n),[{match:t.variablePattern("x_7"),guards:[i=>Erlang["orelse/2"](l=>Erlang["=:=/2"](l.vars.x_7,t.atom("false")),l=>Erlang["=:=/2"](l.vars.x_7,t.atom("nil")),i)],body:i=>t.atom("nil")},{match:t.matchPlaceholder(),guards:[],body:i=>t.list([t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("script"),t.list([t.tuple([t.bitstring("async"),t.list([])]),t.tuple([t.bitstring("src"),t.list([t.tuple([t.atom("expression"),t.tuple([Elixir_Hologram_Router_Helpers["asset_path/1"](t.bitstring("hologram/runtime.js"))])])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
`)])])}],n),t.tuple([t.atom("text"),t.bitstring(`

`)]),e.case(e.case(e.dotOperator(n.vars.vars_0,t.atom("page_mounted?")),[{match:t.variablePattern("x_8"),guards:[i=>Erlang["orelse/2"](l=>Erlang["=:=/2"](l.vars.x_8,t.atom("false")),l=>Erlang["=:=/2"](l.vars.x_8,t.atom("nil")),i)],body:i=>t.atom("true")},{match:t.matchPlaceholder(),guards:[],body:i=>t.atom("false")}],n),[{match:t.atom("false"),guards:[],body:i=>t.atom("nil")},{match:t.atom("true"),guards:[],body:i=>t.list([t.tuple([t.atom("text"),t.bitstring(`
  `)]),t.tuple([t.atom("element"),t.bitstring("script"),t.list([t.tuple([t.bitstring("async"),t.list([])]),t.tuple([t.bitstring("src"),t.list([t.tuple([t.atom("expression"),t.tuple([Elixir_Hologram_Router_Helpers["page_bundle_path/1"](e.dotOperator(i.vars.vars_0,t.atom("page_digest")))])])])])]),t.list([])]),t.tuple([t.atom("text"),t.bitstring(`
`)])])}],n)]))}],s),e.updateVarsToMatchedValues(s),globalThis.Hologram.return)}])};globalThis.Hologram.pageScriptLoaded=!0;document.dispatchEvent(new CustomEvent("hologram:pageScriptLoaded"));console.debug("Hologram: page script executed in",r.diff(u));})();
//# sourceMappingURL=page-445a180c7ec1b89a3fe4a43bb2e9d57a.js.map
