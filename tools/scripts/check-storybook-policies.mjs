const controlTypes = new Set(['boolean', 'range', 'radio', 'select', 'text']);

export function accessibility(parameters) {
  const a11y = parameters.get('a11y')?.object();
  if (a11y) {
    for (const name of ['disable', 'manual']) {
      if (a11y.has(name) && a11y.get(name).scalar() !== false)
        throw new Error(`a11y.${name} cannot disable automatic accessibility checks`);
    }
    if (a11y.has('test') && a11y.get('test').scalar() !== 'error')
      throw new Error('a11y.test must remain error');
    const config = a11y.get('config')?.object();
    if (config?.has('runOnly'))
      throw new Error('a11y runOnly cannot narrow the accessibility gate');
    for (const rule of config?.get('rules')?.array() ?? []) {
      const enabled = rule.object().get('enabled');
      if (enabled && enabled.scalar() !== true) throw new Error('a11y rules cannot be disabled');
    }
    const options = a11y.get('options')?.object();
    if (options?.has('runOnly'))
      throw new Error('a11y runOnly cannot narrow the accessibility gate');
    for (const rule of options?.get('rules')?.object().values() ?? []) {
      const enabled = rule.object().get('enabled');
      if (enabled && enabled.scalar() !== true) throw new Error('a11y rules cannot be disabled');
    }
  }
  const test = parameters.get('test')?.object();
  for (const name of ['disable', 'skip']) {
    if (test?.has(name) && test.get(name).scalar() !== false)
      throw new Error(`test.${name} cannot disable story checks`);
  }
}

export function tags(value) {
  const entries = value?.strings() ?? [];
  if (entries.includes('!test')) throw new Error('Story test tags cannot opt out of the test gate');
  return entries;
}

export function controls(parameters, argTypes) {
  const setting = parameters.get('controls')?.object();
  if (!setting) return [];
  const disabled = setting.get('disable')?.scalar();
  if (typeof disabled !== 'boolean')
    throw new Error('controls must declare disable: true or disable: false');
  const include = setting.get('include')?.strings();
  if (disabled) {
    if (include) throw new Error('disabled controls cannot declare an include list');
    return [];
  }
  if (!include?.length)
    throw new Error('enabled controls require a non-empty literal include list');
  if (!argTypes) throw new Error('enabled controls require explicit argTypes');
  const errors = [];
  for (const name of include) {
    try {
      if (!argTypes.has(name))
        throw new Error(`included control "${name}" is missing from argTypes`);
      const definition = argTypes.get(name).object(),
        control = definition.get('control');
      let type, range;
      try {
        type = control?.scalar();
      } catch {
        range = control?.object();
        type = range?.get('type')?.scalar();
      }
      if (!controlTypes.has(type))
        throw new Error(`control "${name}" must use boolean, range, radio, select, or text`);
      if (type === 'select' || type === 'radio') {
        let choices;
        try {
          choices = definition
            .get('options')
            ?.array()
            .map((choice) => choice.scalar());
        } catch {
          choices = null;
        }
        if (
          !choices?.length ||
          choices.some(
            (choice) =>
              !['string', 'number', 'boolean'].includes(typeof choice) ||
              (typeof choice === 'number' && !Number.isFinite(choice)),
          )
        )
          throw new Error(`control "${name}" requires finite options`);
      }
      if (type === 'range') {
        let min, max, step;
        try {
          min = range?.get('min')?.scalar();
          max = range?.get('max')?.scalar();
          step = range?.get('step')?.scalar();
        } catch {
          min = max = step = NaN;
        }
        if (
          ![min, max, step].every(
            (number) => typeof number === 'number' && Number.isFinite(number),
          ) ||
          min >= max ||
          step <= 0
        )
          throw new Error(
            `range control "${name}" requires numeric min, max, and step with min < max and step > 0`,
          );
      }
    } catch (error) {
      errors.push(error.message);
    }
  }
  for (const name of argTypes.keys()) {
    if (!include.includes(name))
      errors.push(`argType "${name}" must be listed in controls.include`);
  }
  return errors;
}

export function storyPolicy(meta, story) {
  tags(story.get('tags'));
  const parameters = story.get('parameters')?.object() ?? new Map();
  accessibility(parameters);
  if (parameters.has('controls') || story.has('argTypes'))
    throw new Error('story-level controls cannot bypass the meta safety policy');
  if (
    !story.has('component') &&
    !story.has('render') &&
    !meta.has('component') &&
    !meta.has('render')
  )
    throw new Error('A story must inherit a component or render function');
}
