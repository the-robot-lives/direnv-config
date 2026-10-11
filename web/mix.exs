defmodule DirenvConfigWeb.MixProject do
  use Mix.Project

  def project do
    [
      app: :direnv_config_web,
      version: "0.1.0",
      elixir: "~> 1.18",
      elixirc_paths: elixirc_paths(Mix.env()),
      compilers: Mix.compilers() ++ [:hologram],
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      mod: {DirenvConfigWeb.Application, []},
      extra_applications: [:logger, :runtime_tools]
    ]
  end

  # Hologram pages/components live in the top-level `app/` directory.
  defp elixirc_paths(:test), do: ["lib", "app", "test/support"]
  defp elixirc_paths(_), do: ["lib", "app"]

  # Landing site only — no DB, no auth, no S3.
  defp deps do
    [
      {:phoenix, "~> 1.8"},
      {:phoenix_html, "~> 4.0"},
      {:bandit, "~> 1.0"},
      {:jason, "~> 1.4"},
      {:hologram, "~> 0.10.0"}
    ]
  end
end
