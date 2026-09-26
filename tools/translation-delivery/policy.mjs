import path from 'node:path';

const WEIGHT_EXTENSION = /\.(?:gguf|safetensors|ckpt|pth)$/iu;
const TEST_DATA_PATH = /(?:^|\/)(?:\.cache|eval|tests|fixtures|dist-native-e2e)(?:\/|$)/u;
const TEST_MODULE_PATH = /(?:^|\/)(?:native-e2e|tests|fixtures)(?:\/|$)|\.(?:test|spec)\.[cm]?[jt]sx?$/u;

export function assertNoTranslationPayload(files) {
  for (const file of files) {
    const normalized = file.replaceAll('\\', '/');
    if (WEIGHT_EXTENSION.test(normalized) || TEST_DATA_PATH.test(normalized)) {
      throw new Error(`Translation weights or test data must not be bundled: ${file}`);
    }
  }
}

export function assertProductionFrontend(chunks, files) {
  assertNoTranslationPayload(files);
  for (const chunk of chunks) {
    for (const module of chunk.modules) {
      if (module.renderedBytes > 0 && TEST_MODULE_PATH.test(module.id.replaceAll('\\', '/'))) {
        throw new Error(`Test code was emitted in the production frontend: ${module.id}`);
      }
    }
  }
}

export function assertProductionResources(config) {
  const resources = config.bundle?.resources ?? [];
  if (!Array.isArray(resources)) throw new Error('Production resources need an explicit array');
  assertNoTranslationPayload(resources);
  for (const resource of resources) {
    if (!resource.startsWith('binaries/') || resource.includes('**') || path.isAbsolute(resource)) {
      throw new Error(`Production resources must use bounded media-tool paths: ${resource}`);
    }
  }
}
