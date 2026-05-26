defmodule DirenvConfig.MixProject do
  use Mix.Project

  @version "0.1.0"

  def project do
    [
      app: :direnv_config,
      version: @version,
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      package: package(),
      description: "Elixir SDK for direnv-config (dc) — read YAML-backed directory configuration",
      source_url: "https://github.com/noizu/direnv-config"
    ]
  end

  def application do
    [extra_applications: [:logger, :crypto]]
  end

  defp deps do
    [
      {:yaml_elixir, "~> 2.9"},
      {:ex_doc, "~> 0.31", only: :dev, runtime: false},
      {:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false}
    ]
  end

  defp package do
    [
      name: "direnv_config",
      licenses: ["MIT"],
      links: %{"GitHub" => "https://github.com/noizu/direnv-config"}
    ]
  end
end
