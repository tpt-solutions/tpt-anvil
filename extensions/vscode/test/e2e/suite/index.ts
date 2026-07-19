// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

import * as path from 'path';

import { glob } from 'glob';
import Mocha from 'mocha';

export function run(): Promise<void> {
    const mocha = new Mocha({ ui: 'tdd', color: true, timeout: 20000 });
    const testsRoot = __dirname;

    return new Promise((resolve, reject) => {
        glob('**/*.test.js', { cwd: testsRoot })
            .then((files: string[]) => {
                files.forEach((f) => mocha.addFile(path.resolve(testsRoot, f)));
                try {
                    mocha.run((failures: number) => {
                        if (failures > 0) {
                            reject(new Error(`${failures} E2E tests failed.`));
                        } else {
                            resolve();
                        }
                    });
                } catch (err) {
                    reject(err as Error);
                }
            })
            .catch((err: unknown) => reject(err as Error));
    });
}
