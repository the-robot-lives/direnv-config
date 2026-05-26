defmodule DirenvConfig.Version do
  @spec read(String.t()) :: non_neg_integer()
  def read(store_path) do
    path = Path.join(store_path, ".version")

    case File.read(path) do
      {:ok, content} ->
        case Integer.parse(String.trim(content)) do
          {n, _} when n >= 0 -> n
          _ -> 0
        end

      {:error, _} ->
        0
    end
  end
end
