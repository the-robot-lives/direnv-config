<?php

declare(strict_types=1);

namespace Noizu\DirenvConfig\Backend;

use Noizu\DirenvConfig\Exception\ConfigNotFoundException;
use Noizu\DirenvConfig\PathExpression;
use Symfony\Component\Yaml\Yaml;

final readonly class NativeBackend implements BackendInterface
{
    public function __construct(
        private string $storePath,
    ) {}

    public function get(string $config, ?string $path = null): mixed
    {
        $activePath = $this->storePath . '/' . $config . '/.active';
        if (!file_exists($activePath)) {
            throw new ConfigNotFoundException("Config not found: {$config}");
        }

        $content = file_get_contents($activePath);
        $root = Yaml::parse($content);

        if ($path === null || $path === '') {
            return $root;
        }

        return PathExpression::resolve($root, $path);
    }

    /** @return string[] */
    public function listConfigs(): array
    {
        $metaPath = $this->storePath . '/.meta';
        if (!file_exists($metaPath)) {
            return [];
        }

        $content = file_get_contents($metaPath);
        $meta = Yaml::parse($content);

        return $meta['configs'] ?? [];
    }
}
