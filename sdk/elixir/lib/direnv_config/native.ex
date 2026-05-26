defmodule DirenvConfig.Native do
  @spec get(String.t(), String.t(), String.t() | nil) :: {:ok, term()} | :error
  def get(store_path, config, path \\ nil) do
    active_file = Path.join([store_path, config, ".active"])

    case YamlElixir.read_from_file(active_file) do
      {:ok, data} ->
        resolve(data, path)

      {:error, _} ->
        :error
    end
  end

  @spec list_configs(String.t()) :: {:ok, [String.t()]} | :error
  def list_configs(store_path) do
    meta_file = Path.join(store_path, ".meta")

    case YamlElixir.read_from_file(meta_file) do
      {:ok, %{"configs" => configs}} when is_list(configs) ->
        {:ok, configs}

      _ ->
        :error
    end
  end

  defp resolve(data, nil), do: {:ok, data}
  defp resolve(data, path), do: DirenvConfig.Path.get(data, path)
end
