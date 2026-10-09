# pyrs-utils — Biblioteca de utils Rust com sintaxe próxima ao Python

> Documento de **planejamento**. Nada aqui está implementado ainda (exceto `input()`/`try_input()`). O objetivo é reduzir boilerplate do Rust e aproximar a sintaxe do Python, mantendo performance e segurança de memória do Rust.

---

## ✅ Já implementado

### `input()` / `try_input()` / macros `input!` / `try_input!`
- `input::<T>(prompt) -> T` — genérico sobre `FromStr`, dá panic em EOF/parse inválido (equivalente a exceção Python não tratada).
- `try_input::<T>(prompt) -> Result<T, InputError<T::Err>>` — versão sem panic.
- **Não será alterado.**

---

## 📋 Planejado — Sequências e iteração

| Função | Assinatura (planejada) | Equivalente Python |
|---|---|---|
| `range` | `range(stop)` / `range(start, stop)` / `range(start, stop, step)` — suporta step negativo | `range()` |
| `enumerate_from` | `enumerate_from(iter, start)` | `enumerate(iter, start)` |
| `zip_longest` | `zip_longest(a, b, fill)` | `itertools.zip_longest` |

---

## 📋 Planejado — Listas, Arrays e Matrizes

| Função/Método | Assinatura (planejada) | Equivalente Python |
|---|---|---|
| `append` | `lista.append(valor)` | `list.append()` |
| `pop` | `lista.pop()` / `lista.pop(indice)` | `list.pop()` |
| `insert` | `lista.insert(indice, valor)` | `list.insert()` |
| `remove` | `lista.remove(valor)` | `list.remove()` |
| `matrix` | `matrix(linhas, colunas, valor_padrao)` | lista de listas `[[0]*c for _ in range(l)]` |
| `matrix.get/set` | `m.get(i, j)` / `m.set(i, j, valor)` | `m[i][j]` |
| `transpose` | `m.transpose()` | `zip(*m)` / numpy `.T` |
| `flatten` | `flatten(matriz)` | lista comprehension aninhada |
| `list!` (macro) | `list![x*2 for x in 0..10 if x % 2 == 0]` | list comprehension |
| `dict!` / `set!` (macros) | açúcar sintático para `HashMap`/`HashSet` | `{}` / `set()` |

---

## 📋 Planejado — Built-ins gerais

| Função | Assinatura (planejada) | Equivalente Python |
|---|---|---|
| `len` | `len(x)` — genérico (str, Vec, slice, HashMap) | `len()` |
| `min` / `max` | `min(iter)` / `max(iter)` | `min()` / `max()` |
| `sorted` | `sorted(iter) -> Vec` (não consome/muta original) | `sorted()` |
| `reversed` | `reversed(iter)` | `reversed()` |
| `type_of` | `type_of(x) -> &str` (via `std::any::type_name`, uso em debug apenas) | `type()` |

---

## 📋 Planejado — Módulo de aritmética (estilo Python, documentação uniforme)

Cada função documentada no mesmo padrão: descrição curta, tipo genérico aceito, comportamento em overflow/divisão por zero.

| Função | Assinatura (planejada) | Equivalente Python |
|---|---|---|
| `sum` | `sum(iter) -> T` | `sum()` |
| `sub` | `sub(iter) -> T` (subtração sequencial) | — (não existe built-in, mas comum pedir) |
| `mult` | `mult(iter) -> T` (produtório) | `math.prod()` |
| `div` | `div(a, b) -> Result<T, DivError>` (protege divisão por zero) | `a / b` (com `ZeroDivisionError`) |
| `pow` | `pow(base, expoente) -> T` | `**` / `pow()` |
| `root` | `root(x, n) -> f64` (raiz n-ésima; caso especial `sqrt`) | `x ** (1/n)` / `math.sqrt()` |
| `min` / `max` | *(já listados acima, mas fazem parte do mesmo módulo conceitual)* | `min()` / `max()` |

> Nota: `div`, `pow` e `root` precisam decidir explicitamente o comportamento de erro (overflow, divisão por zero, raiz negativa de índice par) — em Rust isso não deve dar panic silencioso, e sim `Result` ou variante documentada, diferente do runtime error do Python.

---

## 📋 Planejado — Strings

| Função | Assinatura (planejada) | Equivalente Python |
|---|---|---|
| `split` | `split(s, sep) -> Vec<&str>` | `str.split()` |
| `strip` | `strip(s) -> &str` | `str.strip()` |
| `join` | `join(sep, iter) -> String` | `sep.join(iter)` |

---

## 📋 Planejado — Orientação a objetos (`class!` macro, estilo C++)

- Macro procedural `class! { Nome { campos...; métodos... } }` que expande internamente para `struct` + `impl` (sintaxe única, sem separar declaração de implementação).
- Construtor implícito gerado a partir dos campos (`new()` automático).
- **Herança**: não existe herança de dados nativa em Rust (design intencional). Planejar:
  - Composição + `Deref`/`DerefMut` para simular acesso herdado a campos.
  - `trait` com métodos default para herança de comportamento.
  - Palavra-chave `extends` no macro, expandindo para composição + delegação automática.
- **Polimorfismo**: via `dyn Trait` ou genéricos `impl Trait`; o macro pode gerar automaticamente o `trait` correspondente aos métodos públicos declarados.
- ⚠️ Ponto de atenção: essa é a parte de maior custo de implementação (macro procedural, não `macro_rules!`) e a que mais tensiona com "zero-cost abstraction" se malfeita (vtable/indireção onde Rust normalmente evita).

---

## 🗂 Ordem sugerida de implementação (a discutir)

1. Built-ins simples (`len`, `min`, `max`, `sorted`, `reversed`)
2. `range` com/sem step
3. Listas/arrays/matrizes (`append`, `pop`, `matrix`, `list!`)
4. Módulo de aritmética (`sum`, `sub`, `mult`, `div`, `pow`, `root`)
5. Strings
6. `class!` macro (OOP estilo C++)

*(Escopo de `hashlib`/criptografia e de `scapy`/redes foi descartado — fora do escopo desta biblioteca.)*
