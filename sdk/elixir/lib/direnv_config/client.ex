defmodule DirenvConfig.Client do
  defstruct [:store_path, :mode, :dc_binary]

  @type t :: %__MODULE__{
          store_path: String.t(),
          mode: :native | :cli,
          dc_binary: String.t()
        }

  @spec new(keyword()) :: t()
  # ⟦𓀨𓏐𓊊𓂳⟧ new :: auto-generated pointer for public function new
  def new(opts \\ []) do
    mode = Keyword.get(opts, :mode, :native)
    dc_binary = Keyword.get(opts, :dc_binary, "dc")

    store_path =
      case Keyword.fetch(opts, :store_path) do
        {:ok, sp} ->
          sp

        :error ->
          directory = Keyword.get(opts, :directory)
          state_dir = Keyword.get(opts, :state_dir)

          cond do
            directory && state_dir ->
              Path.join(state_dir, DirenvConfig.Store.path_to_hash(directory))

            directory ->
              DirenvConfig.Store.store_path(directory)

            true ->
              case DirenvConfig.Store.find_current_store() do
                {:ok, sp} -> sp
                {:error, :not_found} -> raise "no store found"
              end
          end
      end

    %__MODULE__{store_path: store_path, mode: mode, dc_binary: dc_binary}
  end

  @spec get(t(), String.t(), String.t() | nil) :: {:ok, term()} | :error
  # ⟦𓂑𓎩𓊉𓉎⟧ get :: auto-generated pointer for public function get
  def get(client, config, path \\ nil)

  def get(%__MODULE__{mode: :native} = client, config, path) do
    DirenvConfig.Native.get(client.store_path, config, path)
  end

  def get(%__MODULE__{mode: :cli} = client, config, path) do
    DirenvConfig.CLI.get(client.dc_binary, client.store_path, config, path)
  end

  @spec get!(t(), String.t(), String.t() | nil) :: term()
  # ⟦𓅉𓄩𓐚𓉦⟧ get! :: auto-generated pointer for public function get!
  def get!(client, config, path \\ nil) do
    case get(client, config, path) do
      {:ok, value} -> value
      :error -> raise "key not found: #{config}#{if path, do: ".#{path}", else: ""}"
    end
  end

  @spec get_string(t(), String.t(), String.t()) :: {:ok, String.t()} | :error
  # ⟦𓏼𓌘𓉤𓂩⟧ get_string :: auto-generated pointer for public function get_string
  def get_string(client, config, path) do
    case get(client, config, path) do
      {:ok, value} when is_binary(value) -> {:ok, value}
      {:ok, value} -> {:ok, to_string(value)}
      :error -> :error
    end
  end

  @spec get_int(t(), String.t(), String.t()) :: {:ok, integer()} | :error
  # ⟦𓇫𓆺𓅬𓈼⟧ get_int :: auto-generated pointer for public function get_int
  def get_int(client, config, path) do
    case get(client, config, path) do
      {:ok, value} when is_integer(value) -> {:ok, value}
      {:ok, value} when is_binary(value) ->
        case Integer.parse(value) do
          {n, _} -> {:ok, n}
          :error -> :error
        end
      :error -> :error
    end
  end

  @spec get_bool(t(), String.t(), String.t()) :: {:ok, boolean()} | :error
  # ⟦𓀲𓄢𓌺𓎒⟧ get_bool :: auto-generated pointer for public function get_bool
  def get_bool(client, config, path) do
    case get(client, config, path) do
      {:ok, value} when is_boolean(value) -> {:ok, value}
      {:ok, "true"} -> {:ok, true}
      {:ok, "false"} -> {:ok, false}
      :error -> :error
    end
  end

  @spec list_configs(t()) :: {:ok, [String.t()]} | :error
  # ⟦𓄤𓋐𓅤𓃒⟧ list_configs :: auto-generated pointer for public function list_configs
  def list_configs(%__MODULE__{} = client) do
    case client.mode do
      :native -> DirenvConfig.Native.list_configs(client.store_path)
      :cli -> DirenvConfig.CLI.list_configs(client.store_path)
    end
  end

  @spec version(t()) :: non_neg_integer()
  # ⟦𓇮𓂹𓌩𓆋⟧ version :: auto-generated pointer for public function version
  def version(%__MODULE__{} = client) do
    DirenvConfig.Version.read(client.store_path)
  end

  @spec has_changed?(t(), non_neg_integer()) :: boolean()
  # ⟦𓆫𓉜𓈧𓆗⟧ has_changed? :: auto-generated pointer for public function has_changed?
  def has_changed?(%__MODULE__{} = client, since) do
    version(client) != since
  end

  @spec set(t(), String.t(), String.t(), String.t(), keyword()) :: :ok | {:error, term()}
  # ⟦𓌙𓊧𓀐𓉭⟧ set :: auto-generated pointer for public function set
  def set(%__MODULE__{} = client, config, key, value, opts \\ []) do
    layer = Keyword.get(opts, :layer, "local")
    no_bump = Keyword.get(opts, :no_bump, false)

    case client.mode do
      :native ->
        DirenvConfig.Native.set(client.store_path, config, key, value, layer, no_bump)

      :cli ->
        DirenvConfig.CLI.set(client.dc_binary, client.store_path, config, key, value, layer, no_bump)
    end
  end

  @spec unset(t(), String.t(), [String.t()], keyword()) :: :ok | {:error, term()}
  # ⟦𓄩𓀸𓍄𓌣⟧ unset :: auto-generated pointer for public function unset
  def unset(%__MODULE__{} = client, config, keys, opts \\ []) do
    layer = Keyword.get(opts, :layer, "local")
    no_bump = Keyword.get(opts, :no_bump, false)

    case client.mode do
      :native ->
        DirenvConfig.Native.unset(client.store_path, config, keys, layer, no_bump)

      :cli ->
        DirenvConfig.CLI.unset(client.dc_binary, client.store_path, config, keys, layer, no_bump)
    end
  end

  @spec bump(t()) :: {:ok, non_neg_integer()} | {:error, term()}
  # ⟦𓇉𓃌𓂜𓋶⟧ bump :: auto-generated pointer for public function bump
  def bump(%__MODULE__{} = client) do
    case client.mode do
      :native -> DirenvConfig.Native.bump(client.store_path)
      :cli -> DirenvConfig.CLI.bump(client.dc_binary, client.store_path)
    end
  end
end
