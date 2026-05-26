defmodule DirenvConfig.NativeTest do
  use ExUnit.Case, async: true

  @fixtures_dir Path.expand("../../../contract-tests/fixtures", __DIR__)

  describe "get/3 with simple-store" do
    @store Path.join(@fixtures_dir, "simple-store")

    test "reads a simple string" do
      assert DirenvConfig.Native.get(@store, "cluster", "name") == {:ok, "noizu"}
    end

    test "reads nested string" do
      assert DirenvConfig.Native.get(@store, "cluster", "node_pool.instance_type") ==
               {:ok, "m5.xlarge"}
    end

    test "reads integer" do
      assert DirenvConfig.Native.get(@store, "cluster", "port") == {:ok, 6443}
    end

    test "reads boolean" do
      assert DirenvConfig.Native.get(@store, "cluster", "enabled") == {:ok, true}
    end

    test "reads entire config as map" do
      assert {:ok, data} = DirenvConfig.Native.get(@store, "cluster")
      assert is_map(data)
      assert Map.has_key?(data, "name")
      assert Map.has_key?(data, "node_pool")
    end

    test "missing key returns error" do
      assert DirenvConfig.Native.get(@store, "cluster", "nonexistent") == :error
    end
  end

  describe "get/3 with nested-store" do
    @nested_store Path.join(@fixtures_dir, "nested-store")

    test "array index" do
      assert DirenvConfig.Native.get(@nested_store, "app", "endpoints[0].host") ==
               {:ok, "api.example.com"}
    end

    test "wildcard" do
      assert {:ok, hosts} =
               DirenvConfig.Native.get(@nested_store, "app", "endpoints[*].host")

      assert hosts == ["api.example.com", "internal.example.com", "backup.example.com"]
    end

    test "length" do
      assert DirenvConfig.Native.get(@nested_store, "app", "endpoints.length") == {:ok, 3}
    end

    test "chained brackets" do
      assert DirenvConfig.Native.get(@nested_store, "app", "matrix[0][1]") == {:ok, 2}
    end
  end

  describe "list_configs/1" do
    test "simple store" do
      assert DirenvConfig.Native.list_configs(Path.join(@fixtures_dir, "simple-store")) ==
               {:ok, ["cluster"]}
    end

    test "nested store" do
      assert DirenvConfig.Native.list_configs(Path.join(@fixtures_dir, "nested-store")) ==
               {:ok, ["app"]}
    end
  end
end
