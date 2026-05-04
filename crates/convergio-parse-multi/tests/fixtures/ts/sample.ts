// Fixture for convergio-parse-multi TypeScript parser tests (F2-2).
// Each declaration exercises one item_kind in the NodeKind taxonomy.

function greet(name: string): string {
    return `Hello, ${name}!`;
}

class Animal {
    name: string;
    constructor(name: string) {
        this.name = name;
    }
    speak(): void {}
}

interface Describable {
    describe(): string;
}

type Point = {
    x: number;
    y: number;
};

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

export function exportedGreet(name: string): string {
    return greet(name);
}

export class ExportedAnimal extends Animal {}

export interface ExportedDescribable extends Describable {
    label: string;
}

export type ExportedPoint = Point & { z: number };

export enum ExportedColor {
    Red,
    Green,
    Blue,
}
