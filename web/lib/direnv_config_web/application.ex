defmodule DirenvConfigWeb.Application do
  use Application

  @impl true
  def start(_type, _args) do
    children = [
      {Phoenix.PubSub, name: DirenvConfigWeb.PubSub},
      DirenvConfigWeb.Endpoint
    ]

    opts = [strategy: :one_for_one, name: DirenvConfigWeb.Supervisor]
    Supervisor.start_link(children, opts)
  end
end
