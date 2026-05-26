<?php

declare(strict_types=1);

namespace Noizu\DirenvConfig\Tests;

use Noizu\DirenvConfig\Backend\NativeBackend;
use Noizu\DirenvConfig\Exception\ConfigNotFoundException;
use PHPUnit\Framework\TestCase;

final class NativeBackendTest extends TestCase
{
    private static function fixturesDir(): string
    {
        return dirname(__DIR__, 2) . '/contract-tests/fixtures';
    }

    public function testGetSimpleString(): void
    {
        $backend = new NativeBackend(self::fixturesDir() . '/simple-store');
        $this->assertSame('noizu', $backend->get('cluster', 'name'));
    }

    public function testGetNestedString(): void
    {
        $backend = new NativeBackend(self::fixturesDir() . '/simple-store');
        $this->assertSame('m5.xlarge', $backend->get('cluster', 'node_pool.instance_type'));
    }

    public function testGetInteger(): void
    {
        $backend = new NativeBackend(self::fixturesDir() . '/simple-store');
        $this->assertSame(6443, $backend->get('cluster', 'port'));
    }

    public function testGetBoolean(): void
    {
        $backend = new NativeBackend(self::fixturesDir() . '/simple-store');
        $this->assertTrue($backend->get('cluster', 'enabled'));
    }

    public function testGetEntireConfig(): void
    {
        $backend = new NativeBackend(self::fixturesDir() . '/simple-store');
        $result = $backend->get('cluster');
        $this->assertIsArray($result);
        $this->assertArrayHasKey('name', $result);
        $this->assertArrayHasKey('port', $result);
    }

    public function testGetWithWildcard(): void
    {
        $backend = new NativeBackend(self::fixturesDir() . '/nested-store');
        $result = $backend->get('app', 'endpoints[*].host');
        $this->assertSame(['api.example.com', 'internal.example.com', 'backup.example.com'], $result);
    }

    public function testGetMissingPath(): void
    {
        $backend = new NativeBackend(self::fixturesDir() . '/simple-store');
        $this->assertNull($backend->get('cluster', 'nonexistent'));
    }

    public function testListConfigs(): void
    {
        $backend = new NativeBackend(self::fixturesDir() . '/simple-store');
        $this->assertSame(['cluster'], $backend->listConfigs());
    }

    public function testGetMissingConfig(): void
    {
        $backend = new NativeBackend(self::fixturesDir() . '/simple-store');
        $this->expectException(ConfigNotFoundException::class);
        $backend->get('nonexistent');
    }
}
