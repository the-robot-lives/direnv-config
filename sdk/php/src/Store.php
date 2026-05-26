<?php

declare(strict_types=1);

namespace Noizu\DirenvConfig;

use Noizu\DirenvConfig\Exception\StoreNotFoundException;

final class Store
{
    public static function stateDir(): string
    {
        $xdg = $_ENV['XDG_STATE_HOME'] ?? getenv('XDG_STATE_HOME');
        if ($xdg && $xdg !== '') {
            return $xdg . '/direnv-config';
        }
        $home = $_ENV['HOME'] ?? getenv('HOME') ?: (PHP_OS_FAMILY === 'Windows' ? $_ENV['USERPROFILE'] ?? getenv('USERPROFILE') : '');
        return $home . '/.local/state/direnv-config';
    }

    public static function pathToHash(string $dir): string
    {
        $stripped = str_starts_with($dir, '/') ? substr($dir, 1) : $dir;
        $name = str_replace('/', '-', $stripped);

        if (strlen($name) <= 200) {
            return $name;
        }

        $hash = hash('sha256', $dir);
        return substr($name, 0, 200) . '-' . substr($hash, 0, 8);
    }

    public static function storePath(string $dir): string
    {
        return self::stateDir() . '/' . self::pathToHash($dir);
    }

    public static function findCurrentStore(?string $startDir = null): string
    {
        $dir = $startDir ?? getcwd();

        while (true) {
            $sp = self::storePath($dir);
            if (is_dir($sp)) {
                return $sp;
            }

            $parent = dirname($dir);
            if ($parent === $dir) {
                break;
            }
            $dir = $parent;
        }

        throw new StoreNotFoundException(
            'No store found for ' . ($startDir ?? getcwd()) . ' (searched all parent directories). Run `dc init` first.'
        );
    }
}
