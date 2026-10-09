# pyrs-utils — Biblioteca de utils Rust com sintaxe próxima ao Python

> Arquivo único `pyrsutils.rs` (v1.3.1), só `std`, sem dependências. Reduz boilerplate do Rust e aproxima a sintaxe do Python, mantendo performance e segurança de memória.
>
> Uso: `mod pyrsutils; use pyrsutils::prelude::*;`

---

## ✅ Implementado

### Entrada
| Item | Assinatura | Python |
|---|---|---|
| `input` | `input::<T>(prompt) -> T` (`T: FromStr`; panic em EOF/parse inválido) | `input()` |
| `try_input` | `try_input::<T>(prompt) -> Result<T, InputError<T::Err>>` | `try/except` |
| `input!` / `try_input!` | macros equivalentes, `()` ou `(prompt)` | — |

**Não será alterado.**

### Sequências (`range_utils`)
| Item | Assinatura | Python |
|---|---|---|
| `range` | `range(stop)` | `range(stop)` |
| `range_from` | `range_from(start, stop)` | `range(start, stop)` |
| `range_step` | `range_step(start, stop, step)` — step negativo ok; panic se `step == 0` | `range(a, b, c)` |

`PyRange` é `Iterator` + `ExactSizeIterator` + `PyLen` (`len(&range(10))` em O(1), sem alocar).

### Listas (`list_utils`, trait `PyList` sobre `Vec<T>`)
| Item | Assinatura | Python |
|---|---|---|
| `append_item` | `v.append_item(x)` (evita conflito com `Vec::append`) | `list.append()` |
| `pop_at` | `v.pop_at(None)` / `v.pop_at(Some(i))` → `Option<T>` | `list.pop()` / `list.pop(i)` |
| `remove_value` | `v.remove_value(&x) -> bool` (por valor) | `list.remove()` |
| `insert` | nativo `Vec::insert(i, x)` (mesma ordem do Python) | `list.insert()` |

### Matriz (`matrix_utils`)
| Item | Assinatura | Python |
|---|---|---|
| `Matrix::new` | `Matrix::new(linhas, colunas, valor)` — armazenamento contíguo | `[[v]*c for _ in range(l)]` |
| `m[(i, j)]` | `Index`/`IndexMut` (leitura e escrita) | `m[i][j]` |
| `get` | `m.get(i, j) -> Option<&T>` | `m[i][j]` com `IndexError` |
| `transpose` | `m.transpose() -> Matrix<T>` | `zip(*m)` |
| `flatten` | `m.flatten() -> Vec<T>` (consome) | comprehension aninhada |

### Built-ins (`builtins`)
| Item | Assinatura | Python |
|---|---|---|
| `len` | `len(&x)` via `PyLen` (`str` conta chars, `[T]`, `HashMap`, `PyRange`) | `len()` |
| `min` / `max` | `-> Option<T>` (`None` se vazio) | `min()` / `max()` |
| `sorted` | `sorted(iter) -> Vec<T>` (panic em `NaN`) | `sorted()` |
| `reversed` | iterador lazy | `reversed()` |
| `type_of` | `type_of(&x) -> &'static str` (só debug) | `type()` |

### Aritmética (`arithmetic`, trait `NumeroPy`: `i8..i128`, `u8..u128`, `f32`, `f64`)
| Item | Assinatura | Python |
|---|---|---|
| `sum` | `sum(iter) -> T` | `sum()` |
| `sub` | `sub(iter) -> Option<T>` (subtração sequencial) | — |
| `mult` | `mult(iter) -> T` | `math.prod()` |
| `div` | `div(a, b) -> Result<T, ArithError>` (`DivisaoPorZero`) | `a / b` |
| `pow` | `pow(base, expoente: u32) -> T` (sem checagem de overflow) | `**` |
| `root` | `root(x, indice) -> Result<f64, ArithError>` (`RaizDeNegativoComIndicePar`) | `math.sqrt()` / `x ** (1/n)` |

### Strings (`string_utils`)
| Item | Assinatura | Python |
|---|---|---|
| `split` | `split(s, sep) -> Vec<&str>` | `str.split()` |
| `strip` | `strip(s) -> &str` | `str.strip()` |
| `join` | `join(sep, iter) -> String` | `sep.join()` |

### Classes (macro `class!`)
```rust
class! {
    struct Contador { valor: i32 }
    impl {
        fn incrementar(&mut self) { self.valor += 1; }
    }
}
let mut c = Contador::new(0);
```
Gera `struct` + `impl` + `new()` automático. Limitações: tudo `pub`; deriva `Debug + Clone`; sem herança.

### Tempo (`time_utils`, v1.1 — só UTC)
| Item | Python |
|---|---|
| `time`, `sleep`, `perf_counter`, `perf_counter_ns` | `time.time/sleep/perf_counter(_ns)` |
| `timeit`, `timeit_repeat`, macro `timeit!` | `timeit.timeit/repeat` |
| `gmtime`, `gmtime_now`, `mktime_utc`, `StructTime` | `time.gmtime`, `calendar.timegm` |
| `strftime` (`%Y %y %m %d %H %M %S %I %p %j %a %A %b %B %Z %F %T %%`) | `time.strftime` |

### Rede (`net_utils`, v1.1 — só `std::net`, bloqueante)
| Item | Python |
|---|---|
| `TcpClient` (`connect`, `connect_timeout`, `send`, `recv`, `recv_into`, `recv_exact`, `recv_string`, `set_timeout`, `set_nodelay`, `peer_addr`, `local_addr`, `close`) | `socket` TCP cliente |
| `TcpServer` (`listen`, `accept`, `incoming`, `local_addr`) | `bind` + `listen` + `accept` |
| `UdpSock` (`bind`, `send_to`, `recv_from`, `recv_from_into`, `set_timeout`, `local_addr`) | `SOCK_DGRAM` |
| `gethostbyname`, `getaddrinfo` | `socket.gethostbyname/getaddrinfo` |

### Sistema de arquivos (`fs_utils`, v1.2 — `AsRef<Path>`, `io::Result`)
| Item | Python |
|---|---|
| `exists`, `isfile`, `isdir`, `islink` | `os.path.*` |
| `listdir`, `mkdir`, `makedirs`, `remove`, `rmdir`, `rmtree`, `rename`, `copy` | `os`, `shutil` |
| `getcwd`, `chdir` | `os.getcwd/chdir` |
| `join_path`, `basename`, `dirname`, `splitext`, `abspath`, `realpath`, `expanduser` | `os.path.*` |
| `walk` (iterador lazy) | `os.walk` |
| `glob` (só `*` e `?`, sem `**`) | `glob.glob` |
| `stat`, `chmod`, `symlink` (`#[cfg(unix)]`), `readlink` | `os.stat/chmod/symlink/readlink` |

---

## 📋 Ainda planejado (não implementado)

| Item | Assinatura planejada | Python |
|---|---|---|
| `enumerate_from` | `enumerate_from(iter, start)` | `enumerate(iter, start)` |
| `zip_longest` | `zip_longest(a, b, fill)` | `itertools.zip_longest` |
| `list!` | `list![x*2 for x in 0..10 if x % 2 == 0]` | list comprehension |
| `dict!` / `set!` | açúcar sintático para `HashMap`/`HashSet` | `{}` / `set()` |
| `class!` com `extends` | composição + `Deref`/`DerefMut` + delegação automática; trait com métodos default | herança |
| `class!` polimorfismo | geração automática de `trait` dos métodos públicos (`dyn Trait` / `impl Trait`) | polimorfismo |

⚠️ `extends`/polimorfismo exigem macro procedural (não `macro_rules!`) e é onde mais há risco de custo (vtable/indireção) contra o princípio de zero-cost.

---

*(Escopo de `hashlib`/criptografia e de `scapy`/redes avançadas foi descartado — fora do escopo desta biblioteca.)*
