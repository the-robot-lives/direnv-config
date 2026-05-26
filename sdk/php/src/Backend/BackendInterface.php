<?php

declare(strict_types=1);

namespace Noizu\DirenvConfig\Backend;

interface BackendInterface
{
    public function get(string $config, ?string $path = null): mixed;

    /** @return string[] */
    public function listConfigs(): array;
}
