defmodule DirenvConfigWeb.MainLayout do
  @moduledoc """
  Root layout for the direnv-config landing page — head (SEO + OG), sticky header
  nav, footer. Dark-first typographic dev-tool theme; tokens in
  /assets/css/site.css. Copy source of truth: web/COPY.md.
  """

  use Hologram.Component

  def template do
    ~HOLO"""
    <html lang="en">
      <head>
        <meta charset="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <title>direnv-config — YAML in. Env vars out.</title>
        <meta
          name="description"
          content="direnv-config (dc) replaces sprawling export VAR=value .envrc files with named, layered YAML configs — deep-merged and flattened into env vars, with versioned history, file-based IPC, and encrypted-at-rest secrets. Rust CLI, MIT licensed."
        />
        <meta property="og:type" content="website" />
        <meta property="og:title" content="direnv-config — YAML in. Env vars out." />
        <meta
          property="og:description"
          content="One-liner .envrc files loading layered YAML: base → env overlay → local → secrets, deep-merged and flattened into env vars. File-based state any process can read or write; encrypted secrets; timestamped history."
        />
        <meta property="og:url" content="https://dirconfig.therobotlives.com/" />
        <meta name="twitter:card" content="summary" />
        <link rel="icon" href="/assets/favicon.svg" type="image/svg+xml" />
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous" />
        <link
          href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&amp;family=JetBrains+Mono:wght@400;500;700&amp;display=swap"
          rel="stylesheet"
        />
        <link rel="stylesheet" href="/assets/css/site.css" />
        <Hologram.UI.Runtime />
      </head>
      <body>
        <a class="skip-link" href="#main">Skip to content</a>

        <header class="site-header">
          <div class="container header-inner">
            <a class="brand" href="#top" aria-label="direnv-config home">
              <span class="brand-mark" aria-hidden="true">dc</span>
              <span class="brand-name">direnv-config</span>
            </a>
            <nav class="site-nav" aria-label="Primary">
              <a href="#why">Why</a>
              <a href="#how">How it works</a>
              <a href="#cli">CLI &amp; trust</a>
              <a href="#faq">FAQ</a>
              <a class="btn btn-primary btn-sm" href="#install">Install</a>
            </nav>
          </div>
        </header>

        <main id="main">
          <slot />
        </main>

        <footer class="site-footer">
          <div class="container footer-inner">
            <p class="footer-brand">direnv-config — a noizu.com project</p>
            <p class="footer-line">
              <a href="https://github.com/the-robot-lives/direnv-config">GitHub</a> ·
              <a href="https://github.com/the-robot-lives/direnv-config/blob/develop/LICENSE">MIT licensed</a> ·
              This site is Elixir Hologram. · © 2026
            </p>
          </div>
        </footer>
      </body>
    </html>
    """
  end
end
