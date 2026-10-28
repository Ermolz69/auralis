import fs from 'node:fs';
import path from 'node:path';
import { inside } from './check-storybook-typescript.mjs';

export function sourceModule(ts, text, file, root, modules = new Map()) {
  const ast = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  if (ast.parseDiagnostics.length) throw new Error('Storybook source contains invalid syntax');
  const module = { ts, ast, file, root, modules, declarations: new Map(), imports: new Map() };
  for (const statement of ast.statements) {
    if (ts.isVariableStatement(statement)) {
      for (const declaration of statement.declarationList.declarations) {
        if (
          ts.isIdentifier(declaration.name) &&
          statement.declarationList.flags & ts.NodeFlags.Const
        )
          module.declarations.set(declaration.name.text, declaration.initializer);
      }
    } else if (ts.isFunctionDeclaration(statement) && statement.name) {
      module.declarations.set(statement.name.text, statement);
    } else if (ts.isImportDeclaration(statement)) {
      const bindings = statement.importClause?.namedBindings;
      if (bindings && ts.isNamedImports(bindings)) {
        for (const entry of bindings.elements) {
          module.imports.set(entry.name.text, {
            source: statement.moduleSpecifier.text,
            name: (entry.propertyName ?? entry.name).text,
          });
        }
      }
    }
  }
  return module;
}

function imported(module, name) {
  const entry = module.imports.get(name);
  if (!entry?.source.startsWith('.')) return null;
  const base = path.resolve(module.root, path.dirname(module.file), entry.source);
  const file = [base, `${base}.ts`, `${base}.tsx`, `${base}.js`, `${base}.mjs`].find(
    (candidate) => fs.existsSync(candidate) && fs.statSync(candidate).isFile(),
  );
  if (
    !file ||
    !inside(module.root, file) ||
    !inside(module.root, fs.realpathSync(file)) ||
    fs.statSync(file).size > 128 * 1024
  ) {
    throw new Error('Finite registry must be a bounded source inside the owned repository');
  }
  if (!module.modules.has(file))
    module.modules.set(
      file,
      sourceModule(module.ts, fs.readFileSync(file, 'utf8'), file, module.root, module.modules),
    );
  const target = module.modules.get(file);
  const node = target.declarations.get(entry.name);
  return node ? new Value(target, node) : null;
}

export class Value {
  constructor(module, node, bindings = new Map(), trail = new Set()) {
    Object.assign(this, { module, node, bindings, trail });
  }
  child(node, bindings = this.bindings, trail = this.trail) {
    return new Value(this.module, node, bindings, trail);
  }
  resolve() {
    const { ts } = this.module;
    let node = this.node;
    while (
      node &&
      (ts.isParenthesizedExpression(node) ||
        ts.isAsExpression(node) ||
        ts.isSatisfiesExpression(node) ||
        ts.isTypeAssertionExpression(node))
    )
      node = node.expression;
    if (!node) throw new Error('Missing static Storybook value');
    if (ts.isIdentifier(node)) {
      if (this.trail.has(node)) throw new Error('Cyclic Storybook metadata');
      const trail = new Set([...this.trail, node]);
      const bound = this.bindings.get(node.text);
      if (bound) return new Value(bound.module, bound.node, bound.bindings, trail).resolve();
      const declaration = this.module.declarations.get(node.text);
      if (declaration) return this.child(declaration, this.bindings, trail).resolve();
      const external = imported(this.module, node.text);
      if (external)
        return new Value(external.module, external.node, external.bindings, trail).resolve();
    }
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression)) {
      const local = this.module.declarations.get(node.expression.text);
      const fnValue = local
        ? this.child(local).resolve()
        : imported(this.module, node.expression.text)?.resolve();
      const fn = fnValue?.node;
      if (fn && (ts.isFunctionDeclaration(fn) || ts.isArrowFunction(fn))) {
        const body = fn.body;
        const expression =
          body &&
          ts.isBlock(body) &&
          body.statements.length === 1 &&
          ts.isReturnStatement(body.statements[0])
            ? body.statements[0].expression
            : body && !ts.isBlock(body)
              ? body
              : null;
        if (expression) {
          if (this.trail.has(fn)) throw new Error('Cyclic Storybook metadata helper');
          const bindings = new Map(fnValue.bindings);
          fn.parameters.forEach((parameter, index) => {
            if (ts.isIdentifier(parameter.name))
              bindings.set(
                parameter.name.text,
                this.child(node.arguments[index] ?? parameter.initializer),
              );
          });
          return new Value(
            fnValue.module,
            expression,
            bindings,
            new Set([...this.trail, fn]),
          ).resolve();
        }
      }
    }
    return this.child(node);
  }
  scalar() {
    const value = this.resolve(),
      { ts } = value.module,
      node = value.node;
    if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return node.text;
    if (ts.isNumericLiteral(node)) return Number(node.text);
    if (node.kind === ts.SyntaxKind.TrueKeyword) return true;
    if (node.kind === ts.SyntaxKind.FalseKeyword) return false;
    if (
      ts.isPrefixUnaryExpression(node) &&
      [ts.SyntaxKind.MinusToken, ts.SyntaxKind.PlusToken].includes(node.operator)
    ) {
      const number = value.child(node.operand).scalar();
      if (typeof number === 'number')
        return node.operator === ts.SyntaxKind.MinusToken ? -number : number;
    }
    throw new Error('Storybook safety values must be statically resolved literals');
  }
  object() {
    const value = this.resolve(),
      { ts } = value.module,
      node = value.node;
    if (!ts.isObjectLiteralExpression(node))
      throw new Error('Storybook safety object cannot be resolved');
    const result = new Map();
    for (const property of node.properties) {
      if (ts.isSpreadAssignment(property)) {
        for (const [key, entry] of value.child(property.expression).object())
          result.set(key, entry);
        continue;
      }
      const name = property.name;
      const key =
        name && ts.isComputedPropertyName(name)
          ? value.child(name.expression).scalar()
          : name?.text;
      if (
        typeof key !== 'string' ||
        ts.isGetAccessorDeclaration(property) ||
        ts.isSetAccessorDeclaration(property)
      )
        throw new Error('Storybook safety property cannot be resolved');
      const expression = ts.isPropertyAssignment(property)
        ? property.initializer
        : ts.isShorthandPropertyAssignment(property)
          ? property.name
          : property;
      result.set(key, value.child(expression));
    }
    return result;
  }
  array() {
    const value = this.resolve(),
      { ts } = value.module,
      node = value.node;
    if (ts.isArrayLiteralExpression(node))
      return node.elements.flatMap((entry) =>
        ts.isSpreadElement(entry) ? value.child(entry.expression).array() : [value.child(entry)],
      );
    if (
      ts.isCallExpression(node) &&
      ts.isPropertyAccessExpression(node.expression) &&
      node.expression.expression.getText() === 'Object' &&
      node.expression.name.text === 'keys' &&
      node.arguments.length === 1
    ) {
      return [...value.child(node.arguments[0]).object().keys()].map((key) =>
        value.child(ts.factory.createStringLiteral(key)),
      );
    }
    throw new Error('Storybook safety arrays must have finite statically resolved values');
  }
  strings() {
    return this.array().map((value) => {
      const scalar = value.scalar();
      if (typeof scalar !== 'string')
        throw new Error('Storybook string list contains a non-string');
      return scalar;
    });
  }
  callable() {
    const value = this.resolve(),
      { ts } = value.module;
    return (
      ts.isArrowFunction(value.node) ||
      ts.isFunctionExpression(value.node) ||
      ts.isFunctionDeclaration(value.node) ||
      ts.isMethodDeclaration(value.node)
    );
  }
}

export function defaultValue(module) {
  const values = module.ast.statements.filter(
    (node) => module.ts.isExportAssignment(node) && !node.isExportEquals,
  );
  if (values.length !== 1) throw new Error('An actual default Storybook export is required');
  return new Value(module, values[0].expression);
}

export function exportedStories(module, meta) {
  const { ts } = module,
    include = meta.get('includeStories')?.strings(),
    exclude = meta.get('excludeStories')?.strings() ?? [];
  const stories = [];
  for (const statement of module.ast.statements) {
    if (
      !ts.isVariableStatement(statement) ||
      !statement.modifiers?.some((modifier) => modifier.kind === ts.SyntaxKind.ExportKeyword)
    )
      continue;
    for (const declaration of statement.declarationList.declarations) {
      const name = declaration.name.text;
      if (
        !name ||
        name === '__namedExportsOrder' ||
        (include && !include.includes(name)) ||
        exclude.includes(name)
      )
        continue;
      stories.push({ name, value: new Value(module, declaration.initializer) });
    }
  }
  return stories;
}
