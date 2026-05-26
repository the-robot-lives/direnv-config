defmodule DirenvConfig.Path do
  @type segment ::
          {:key, String.t()}
          | {:index, integer()}
          | :wildcard
          | :length

  @spec parse(String.t()) :: [segment()]
  def parse(path) when is_binary(path) do
    if path == "" do
      []
    else
      path
      |> String.split(".")
      |> parse_tokens([], false)
    end
  end

  defp parse_tokens([], acc, _has_prev), do: Enum.reverse(acc)

  defp parse_tokens([token | rest], acc, has_prev) do
    if token == "length" and has_prev do
      parse_tokens(rest, [:length | acc], true)
    else
      segments = parse_token(token)
      parse_tokens(rest, Enum.reverse(segments) ++ acc, true)
    end
  end

  defp parse_token(token) do
    case :binary.match(token, "[") do
      :nomatch ->
        [{:key, token}]

      {pos, _} ->
        key_part = binary_part(token, 0, pos)
        bracket_part = binary_part(token, pos, byte_size(token) - pos)

        key_segments = if key_part != "", do: [{:key, key_part}], else: []
        bracket_segments = parse_brackets(bracket_part)

        key_segments ++ bracket_segments
    end
  end

  defp parse_brackets(""), do: []

  defp parse_brackets(rest) do
    case :binary.match(rest, "[") do
      :nomatch ->
        []

      {open, _} ->
        {close, _} = :binary.match(rest, "]")
        inner = binary_part(rest, open + 1, close - open - 1)

        segment =
          case inner do
            "*" -> :wildcard
            _ -> {:index, String.to_integer(inner)}
          end

        remaining = binary_part(rest, close + 1, byte_size(rest) - close - 1)
        [segment | parse_brackets(remaining)]
    end
  end

  @spec get(term(), String.t()) :: {:ok, term()} | :error
  def get(root, path) do
    segments = parse(path)
    get_segments(root, segments)
  end

  defp get_segments(current, []), do: {:ok, current}

  defp get_segments(current, [{:key, key} | rest]) when is_map(current) do
    case Map.fetch(current, key) do
      {:ok, child} -> get_segments(child, rest)
      :error -> :error
    end
  end

  defp get_segments(current, [{:index, idx} | rest]) when is_list(current) do
    case resolve_index(idx, length(current)) do
      {:ok, resolved} -> get_segments(Enum.at(current, resolved), rest)
      :error -> :error
    end
  end

  defp get_segments(current, [:wildcard | rest]) when is_list(current) do
    collected =
      current
      |> Enum.map(fn elem -> get_segments(elem, rest) end)
      |> Enum.filter(fn
        {:ok, _} -> true
        :error -> false
      end)
      |> Enum.map(fn {:ok, v} -> v end)

    {:ok, collected}
  end

  defp get_segments(current, [:length]) when is_list(current), do: {:ok, length(current)}
  defp get_segments(current, [:length]) when is_map(current), do: {:ok, map_size(current)}

  defp get_segments(_current, _segments), do: :error

  defp resolve_index(idx, len) when idx < 0 do
    resolved = len + idx
    if resolved < 0 or resolved >= len, do: :error, else: {:ok, resolved}
  end

  defp resolve_index(idx, len) do
    if idx < 0 or idx >= len, do: :error, else: {:ok, idx}
  end
end
