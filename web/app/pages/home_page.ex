defmodule DirenvConfigWeb.Pages.HomePage do
  @moduledoc """
  direnv-config landing page — single static route "/" per web/COPY.md.
  No client state or actions needed; SSR only, minimal JS (Hologram runtime).
  """

  use Hologram.Page

  route "/"
  layout DirenvConfigWeb.MainLayout

  def init(_params, component, _server) do
    component
  end
end
