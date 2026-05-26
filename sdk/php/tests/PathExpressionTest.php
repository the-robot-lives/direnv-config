<?php

declare(strict_types=1);

namespace Noizu\DirenvConfig\Tests;

use Noizu\DirenvConfig\PathExpression;
use Noizu\DirenvConfig\Segment;
use Noizu\DirenvConfig\SegmentType;
use PHPUnit\Framework\TestCase;

final class PathExpressionTest extends TestCase
{
    public function testParseSimpleKey(): void
    {
        $segments = PathExpression::parse('name');
        $this->assertCount(1, $segments);
        $this->assertSame(SegmentType::Key, $segments[0]->type);
        $this->assertSame('name', $segments[0]->value);
    }

    public function testParseDottedPath(): void
    {
        $segments = PathExpression::parse('node_pool.instance_type');
        $this->assertCount(2, $segments);
        $this->assertSame(SegmentType::Key, $segments[0]->type);
        $this->assertSame('node_pool', $segments[0]->value);
        $this->assertSame(SegmentType::Key, $segments[1]->type);
        $this->assertSame('instance_type', $segments[1]->value);
    }

    public function testParseIndex(): void
    {
        $segments = PathExpression::parse('endpoints[0].host');
        $this->assertCount(3, $segments);
        $this->assertSame(SegmentType::Key, $segments[0]->type);
        $this->assertSame('endpoints', $segments[0]->value);
        $this->assertSame(SegmentType::Index, $segments[1]->type);
        $this->assertSame(0, $segments[1]->value);
        $this->assertSame(SegmentType::Key, $segments[2]->type);
        $this->assertSame('host', $segments[2]->value);
    }

    public function testParseNegativeIndex(): void
    {
        $segments = PathExpression::parse('items[-1]');
        $this->assertCount(2, $segments);
        $this->assertSame(SegmentType::Index, $segments[1]->type);
        $this->assertSame(-1, $segments[1]->value);
    }

    public function testParseWildcard(): void
    {
        $segments = PathExpression::parse('endpoints[*].host');
        $this->assertCount(3, $segments);
        $this->assertSame(SegmentType::Wildcard, $segments[1]->type);
    }

    public function testParseLength(): void
    {
        $segments = PathExpression::parse('endpoints.length');
        $this->assertCount(2, $segments);
        $this->assertSame(SegmentType::Length, $segments[1]->type);
    }

    public function testLengthAsFirstTokenIsKey(): void
    {
        $segments = PathExpression::parse('length');
        $this->assertCount(1, $segments);
        $this->assertSame(SegmentType::Key, $segments[0]->type);
        $this->assertSame('length', $segments[0]->value);
    }

    public function testParseChainedBrackets(): void
    {
        $segments = PathExpression::parse('matrix[0][1]');
        $this->assertCount(3, $segments);
        $this->assertSame(SegmentType::Key, $segments[0]->type);
        $this->assertSame('matrix', $segments[0]->value);
        $this->assertSame(SegmentType::Index, $segments[1]->type);
        $this->assertSame(0, $segments[1]->value);
        $this->assertSame(SegmentType::Index, $segments[2]->type);
        $this->assertSame(1, $segments[2]->value);
    }

    public function testParseEmptyString(): void
    {
        $segments = PathExpression::parse('');
        $this->assertCount(0, $segments);
    }

    public function testResolveSimpleKey(): void
    {
        $data = ['name' => 'noizu', 'port' => 6443];
        $this->assertSame('noizu', PathExpression::resolve($data, 'name'));
    }

    public function testResolveNestedKey(): void
    {
        $data = ['node_pool' => ['instance_type' => 'm5.xlarge']];
        $this->assertSame('m5.xlarge', PathExpression::resolve($data, 'node_pool.instance_type'));
    }

    public function testResolveIndex(): void
    {
        $data = ['items' => ['a', 'b', 'c']];
        $this->assertSame('b', PathExpression::resolve($data, 'items[1]'));
    }

    public function testResolveNegativeIndex(): void
    {
        $data = ['items' => ['a', 'b', 'c']];
        $this->assertSame('c', PathExpression::resolve($data, 'items[-1]'));
    }

    public function testResolveWildcard(): void
    {
        $data = ['items' => [['x' => 1], ['x' => 2], ['x' => 3]]];
        $this->assertSame([1, 2, 3], PathExpression::resolve($data, 'items[*].x'));
    }

    public function testResolveLength(): void
    {
        $data = ['items' => [1, 2, 3]];
        $this->assertSame(3, PathExpression::resolve($data, 'items.length'));
    }

    public function testResolveLengthMap(): void
    {
        $data = ['meta' => ['a' => 1, 'b' => 2]];
        $this->assertSame(2, PathExpression::resolve($data, 'meta.length'));
    }

    public function testResolveMissingKey(): void
    {
        $data = ['name' => 'test'];
        $this->assertNull(PathExpression::resolve($data, 'nonexistent'));
    }

    public function testResolveOutOfBounds(): void
    {
        $data = ['items' => [1, 2]];
        $this->assertNull(PathExpression::resolve($data, 'items[99]'));
    }

    public function testResolveChainedBrackets(): void
    {
        $data = ['matrix' => [[1, 2, 3], [4, 5, 6]]];
        $this->assertSame(2, PathExpression::resolve($data, 'matrix[0][1]'));
        $this->assertSame(6, PathExpression::resolve($data, 'matrix[1][-1]'));
    }
}
