#!/usr/bin/env node
import { chmod } from 'node:fs/promises';
const file = process.argv[2];
if (!file) process.exit(0);
if (process.platform !== 'win32') await chmod(file, 0o755);
