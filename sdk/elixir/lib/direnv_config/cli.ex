defmodule DirenvConfig.CLI do
  @spec get(String.t(), String.t(), String.t(), String.t() | nil) :: {:ok, String.t()} | :error
  def get(dc_binary \\ "dc", store_path, config, path \\ nil) do
    args =
      ["get", config] ++
        (if path, do: [path], else: []) ++
        ["--raw", "--store", store_path]

    case System.cmd(dc_binary, args, stderr_to_stdout: true) do
      {output, 0} -> {:ok, String.trim(output)}
      _ -> :error
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
end
