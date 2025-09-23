import { unzip } from 'node:zlib'
import { join } from 'node:path'
import { mkdir } from 'node:fs/promises'
import { promisify } from 'node:util'
import { fileURLToPath } from 'node:url'

import { Archive } from '@napi-rs/tar'

const root = join(fileURLToPath(import.meta.url), '..')

await mkdir(join(root, 'fixtures'), { recursive: true })

const headers = new Headers()

if (process.env.GITHUB_TOKEN) {
  headers.set('Authorization', `Bearer ${process.env.GITHUB_TOKEN}`)
}

const AFFiNETarBuffer = await fetch(
  `https://github.com/toeverything/AFFiNE/archive/refs/tags/v0.24.2/v0.24.2.tar.gz`,
  {
    headers,
  }
).then((response) => response.arrayBuffer())

const AFFiNETar = await promisify(unzip)(AFFiNETarBuffer)

const AFFiNETarArchive = new Archive(AFFiNETar)

AFFiNETarArchive.unpack(join(root, 'fixtures'))
