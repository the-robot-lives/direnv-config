defmodule DirenvConfigWeb.ErrorHTML do
  @moduledoc """
  Minimal error renderer — plain status messages, no templates needed.
  """

  def render(template, _assigns) do
    Phoenix.Controller.status_message_from_template(template)
  end
end
