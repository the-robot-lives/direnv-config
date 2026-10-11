defmodule DirenvConfigWeb.RequestTest do
  use ExUnit.Case, async: false
  import Phoenix.ConnTest

  @endpoint DirenvConfigWeb.Endpoint

  test "GET /healthz returns ok for k8s probes" do
    conn = get(build_conn(), "/healthz")
    assert conn.status == 200
    assert conn.resp_body == "ok"
  end

  test "GET / renders the landing page with the hero headline" do
    conn = get(build_conn(), "/")
    assert conn.status == 200
    assert conn.resp_body =~ "YAML in. Env vars out."
  end

  test "GET / includes SEO head tags" do
    conn = get(build_conn(), "/")
    body = conn.resp_body

    assert body =~ "<title>direnv-config — YAML in. Env vars out.</title>"
    assert body =~ ~r/name="description" content="direnv-config \(dc\) replaces sprawling/
    assert body =~ "og:title"
  end

  test "GET / includes benefits, why table, how-it-works, CLI, trust, FAQ, and CTA copy" do
    conn = get(build_conn(), "/")
    body = conn.resp_body

    assert body =~ "One-liner .envrc, structured YAML"
    assert body =~ "File-based state is IPC"
    assert body =~ "Secrets, first-class"
    assert body =~ "The .envrc you have vs. the one you could have"
    assert body =~ "Timestamped snapshots on every write"
    assert body =~ "How it works"
    assert body =~ "22 subcommands, one small binary"
    assert body =~ "A published threat model"
    assert body =~ "A published test-health report"
    assert body =~ "Is this just .env files with extra steps?"
    assert body =~ "the license?"
    assert body =~ "make doctor"
    assert body =~ "Install from source"
  end

  test "GET / keeps the install section honest" do
    conn = get(build_conn(), "/")
    body = conn.resp_body

    # Prerequisites stated up front; no one-liner / brew / cargo-install promises.
    assert body =~ "prerequisites: direnv hooked into your shell"
    assert body =~ "not on crates.io or Homebrew yet"
    assert body =~ "Packaged installs are planned"
    refute body =~ ~r/brew install/
    refute body =~ ~r/cargo install dc/
  end

  test "GET / uses neutral example values, not internal hostnames" do
    conn = get(build_conn(), "/")
    body = conn.resp_body

    assert body =~ "name: acme"
    assert body =~ "~/.kube/acme/config"
    # Internal example values from the README/demo must not appear. (The
    # footer's "a noizu.com project" attribution is deliberate public branding.)
    refute body =~ "noizu/"
    refute body =~ "name: noizu"
    refute body =~ "staging."
    refute body =~ "design."
  end

  test "GET / includes footer attribution" do
    conn = get(build_conn(), "/")
    body = conn.resp_body

    assert body =~ "a noizu.com project"
    assert body =~ "https://github.com/the-robot-lives/direnv-config"
    assert body =~ "This site is Elixir Hologram."
  end

  test "static assets are served" do
    conn = get(build_conn(), "/assets/css/site.css")
    assert conn.status == 200
  end
end
