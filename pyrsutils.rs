//! # pyrsutils — v1.3.1
//! #![allow(unused)]
//! Biblioteca única de utilidades que traz pro Rust a ergonomia do dia a
//! dia de Python (e, na seção de "classes", a sintaxe familiar de C++),
//! sem abrir mão de nenhuma garantia do Rust: tipagem estática, checagem
//! em tempo de compilação, zero-cost abstractions, sem GC e sem overhead
//! de runtime. Tudo aqui é **monomorphization** (código especializado por
//! tipo, gerado em compile-time) — nunca dynamic dispatch/`Any` escondido.
//!
//! ## Como este arquivo nasceu (e o que mudou na v1.0)
//!
//! Este arquivo consolida dois pontos de partida:
//! - `pyinput_unificado.rs` — a implementação original e independente de
//!   `input()`/`try_input()`.
//! - a versão estendida que adiciona `range()`, listas, matriz, built-ins
//!   gerais, aritmética, strings e a macro `class!`.
//!
//! Os dois arquivos eram consistentes entre si (o segundo é um
//! *superset* do primeiro, com o mesmo bloco de `input()` copiado sem
//! alterações). Na revisão pra v1.0, os seguintes pontos foram
//! corrigidos/fechados:
//!
//! 1. **Doc do módulo desatualizada.** O comentário de topo descrevia só
//!    `input()`, mas o arquivo já continha 7 outras seções. Reescrito
//!    (este bloco) pra documentar a biblioteca inteira, com exemplo de
//!    uso prático por seção — era o pedido explícito e também um bug de
//!    documentação real (doc que mente sobre o conteúdo do arquivo).
//! 2. **`PyRange` não tinha paridade com `len(range(...))` do Python.**
//!    Em Python, `len(range(10, 0, -2))` funciona (`range` sabe seu
//!    próprio tamanho sem iterar). Aqui, `PyRange` só implementava
//!    `Iterator`, então `range_utils::range(10)` não podia ser usado com
//!    `builtins::len(...)`. Corrigido: `PyRange` agora implementa
//!    `ExactSizeIterator` (cálculo O(1), sem iterar) e `PyLen`, então
//!    `len(&range(10))` já funciona — ver seção 1 e o teste
//!    `range_tem_len_sem_iterar`.
//! 3. **Prelude incompleto.** `PyRange` (o tipo) e o trait `PyLen` não
//!    estavam reexportados — quem desse só `use pyrsutils::prelude::*;`
//!    não conseguia nomear `PyRange` num retorno de função nem
//!    implementar `PyLen` pros próprios tipos. Adicionados ao prelude.
//! 4. **Nome do módulo no comentário do prelude estava desalinhado com o
//!    nome do arquivo entregue.** Ajustado pra `pyrsutils` (o nome deste
//!    arquivo), mantendo a explicação de por que o `use super::` (e não
//!    `crate::`) é proposital.
//!
//! O resto (input/try_input, range, listas, matriz, built-ins,
//! aritmética, strings, `class!`) já compilava limpo e passava nos
//! testes originais — comportamento preservado 1:1, só reorganizado e
//! documentado. Compilado e testado com `rustc 1.75` (edition 2021)
//! antes da entrega: `13` testes originais + `2` novos, todos passando.
//!
//! ## Tabela geral — pyrsutils ↔ Python ↔ C++
//!
//! | Seção            | Python                          | C++ / Rust nativo              | Aqui (`pyrsutils`)                  |
//! |-------------------|----------------------------------|---------------------------------|---------------------------------------|
//! | Entrada           | `input()`                       | `std::cin >>`                   | `input::<T>(prompt)` / `input!()`     |
//! | Ranges             | `range(a, b, c)`                | `for(int i=a;i<b;i+=c)`         | `range`/`range_from`/`range_step`     |
//! | Listas             | `list.append/pop/remove`        | `std::vector::push_back`        | `Vec<T>` + trait `PyList`             |
//! | Matriz             | `[[0]*c for _ in range(l)]`     | `T m[L][C]` / `std::vector<vector<T>>` | `Matrix<T>` (armazenamento contíguo) |
//! | Built-ins          | `len/min/max/sorted/reversed`   | `.size()/std::min/std::sort`    | funções livres em `builtins`          |
//! | Aritmética         | `sum/math.prod/pow/**`          | operadores nativos               | `arithmetic::{sum,sub,mult,div,pow,root}` |
//! | Strings            | `s.split/strip`, `sep.join()`   | `std::string` + boost/manual     | `string_utils::{split,strip,join}`    |
//! | Classe             | `class X: def __init__(self):`  | `class X { campos; métodos; };`  | macro `class! { struct X {...} impl {...} }` |
//! | Tempo              | `time.time/sleep/perf_counter/timeit/strftime` | `std::chrono` (crate) / `clock_gettime` | `time_utils::{time,sleep,perf_counter,timeit,gmtime,strftime}` e macro `timeit!` |
//! | Rede               | `socket` (TCP/UDP/DNS)          | `socket()/bind()/accept()` (POSIX) | `net_utils::{TcpClient,TcpServer,UdpSock,gethostbyname,getaddrinfo}` |
//!
//! ## Uso prático — um exemplo por seção
//!
//! Import único recomendado (traz tudo que você normalmente precisa):
//! ```
//! use pyrsutils::prelude::*;
//! ```
    #![allow(unused)]
//! ### 1) Entrada de dados (equivalente a `input()`)
//! ```
//! // Python:  idade = int(input("Idade: "))
//! let idade: i32 = input("Idade: ");
//!
//! // Sem panic em entrada inválida/EOF — equivalente a um try/except:
//! match try_input::<i32>("Idade: ") {
//!     Ok(v) => println!("ok: {v}"),
//!     Err(e) => println!("entrada inválida: {e:?}"),
//! }
//!
//! // Via macro, com o mesmo comportamento da função:
//! let nome: String = input!("Nome: ");
//! ```
//!
//! ### 2) `range()`
//! ```
//! // Python:  for i in range(0, 10, 2): ...
//! for i in range_step(0, 10, 2) {
//!     println!("{i}");
//! }
//! // len(range(...)) também funciona, sem iterar (O(1)):
//! assert_eq!(len(&range(10)), 10);
//! ```
//!
//! ### 3) Listas
//! ```
//! let mut v = vec![1, 2, 3];
//! v.append_item(4);          // Python: v.append(4)
//! v.pop_at(None);             // Python: v.pop()
//! v.remove_value(&2);         // Python: v.remove(2)
//! ```
//!
//! ### 4) Matriz
//! ```
//! let mut m = Matrix::new(2, 3, 0);   // Python: [[0]*3 for _ in range(2)]
//! m[(0, 1)] = 5;                       // Python: m[0][1] = 5
//! let t = m.transpose();
//! ```
//!
//! ### 5) Built-ins gerais
//! ```
//! len("café");                          // 4 (chars, igual ao Python)
//! min(vec![3, 1, 2]);                   // Some(1)
//! sorted(vec![3, 1, 2]);                // vec![1, 2, 3]
//! reversed(vec![1, 2, 3]);               // iterador 3,2,1
//! ```
//!
//! ### 6) Aritmética
//! ```
//! sum(vec![1, 2, 3]);        // 6      — Python: sum([1,2,3])
//! mult(vec![2, 3, 4]);        // 24     — Python: math.prod([2,3,4])
//! div(10, 0);                  // Err(ArithError::DivisaoPorZero), não panic
//! pow(2, 10);                  // 1024   — Python: 2**10
//! root(9.0, 2.0);              // Ok(3.0) — Python: math.sqrt(9) / 9**0.5
//! ```
//!
//! ### 7) Strings
//! ```
//! split("a,b,c", ",");        // Python: "a,b,c".split(",")
//! strip("  oi  ");             // Python: "  oi  ".strip()
//! join(", ", vec!["a", "b"]);  // Python: ", ".join(["a","b"])
//! ```
//!
//! ### 8) "Classe" (macro `class!`, estilo C++/Python juntos)
//! ```
//! class! {
//!     struct Contador {
//!         valor: i32
//!     }
//!     impl {
//!         fn incrementar(&mut self) {
//!             self.valor += 1;
//!         }
//!         fn valor_atual(&self) -> i32 {
//!             self.valor
//!         }
//!     }
//! }
//!
//! // Python:  c = Contador(0); c.incrementar()
//! // C++:     Contador c(0); c.incrementar();
//! let mut c = Contador::new(0);
//! c.incrementar();
//! assert_eq!(c.valor_atual(), 1);
//! ```
//! ⚠️ Limitações assumidas nesta v1.0 (documentadas, não são bugs
//! escondidos): todo campo/método gerado sai `pub`; o struct sempre
//! deriva `Debug + Clone` (então todo campo precisa implementar os
//! dois); ainda não há suporte a herança (`extends`) — fica para uma
//! v1.1, via composição + `Deref` ou trait com métodos default.

//! ## Novidades da v1.1 (incremental — nada da v1.0 foi alterado)
//!
//! - **Seção 8 — `time_utils`**: `time`, `sleep`, `perf_counter(_ns)`,
//!   `timeit`, `timeit_repeat`, macro `timeit!`, `gmtime`, `gmtime_now`,
//!   `mktime_utc`, `strftime` (UTC). Guia de uso no `//!` do módulo.
//! - **Seção 9 — `net_utils`**: `TcpClient`, `TcpServer`, `UdpSock`,
//!   `gethostbyname`, `getaddrinfo` (só `std::net`). Guia de uso no `//!`
//!   do módulo (servidor de eco, UDP, DNS).
//! - Prelude estendido com os nomes acima; testes novos no módulo
//!   `tests_tempo_rede` (os testes da v1.0 permanecem intactos).
//!
//! ### 9) Tempo (resumo)
//! ```ignore
//! let t0 = perf_counter();
//! let total = timeit!(1_000, { (0..100u64).sum::<u64>() }); // segundos
//! println!("{}", strftime("%Y-%m-%d %H:%M:%S", &gmtime(time() as i64)));
//! ```
//!
//! ### 10) Rede (resumo)
//! ```ignore
//! let mut c = TcpClient::connect(("127.0.0.1", 8080))?;
//! c.send("GET / HTTP/1.0\r\n\r\n")?;
//! let resposta = c.recv_string(4096)?;
//! let ip = gethostbyname("localhost")?;
//! ```

// ============================================================================
// 0) input() / try_input() — equivalente unificado e genérico ao input()
//    do Python, para qualquer T: FromStr (String, i32, f64, bool, char...)
// ============================================================================

use std::fmt::Debug;
use std::io::{self, Write};
use std::str::FromStr;

/// Erro unificado: ou a entrada acabou (EOF), ou o texto não pôde ser
/// convertido para o tipo `T` pedido.
#[derive(Debug)]
pub enum InputError<E> {
    /// Stream fechado antes de qualquer dado (Ctrl+D / Ctrl+Z).
    Eof,
    /// Falha de I/O ao ler stdin.
    Io(io::Error),
    /// O texto lido não é um valor válido do tipo pedido.
    Parse(E),
}

/// Lê uma linha de stdin e remove só o terminador final — sem nenhuma
/// conversão de tipo ainda.
fn read_raw(prompt: &str) -> Result<String, io::Error> {
    if !prompt.is_empty() {
        print!("{prompt}");
        io::stdout().flush()?;
    }

    let mut buffer = String::new();
    let bytes_read = io::stdin().read_line(&mut buffer)?;

    if bytes_read == 0 {
        return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "EOF"));
    }

    if buffer.ends_with('\n') {
        buffer.pop();
        if buffer.ends_with('\r') {
            buffer.pop();
        }
    }

    Ok(buffer)
}

/// **A função**: equivalente unificado a `input()` do Python para
/// qualquer tipo `T: FromStr` — `String`, `i32`, `u32`, `i64`, `f32`,
/// `f64`, `bool`, `char`, ou qualquer tipo seu com `FromStr` implementado.
///
/// O tipo `T` é inferido a partir de onde o valor é usado (variável
/// anotada, ou contexto da chamada) — não existe overload em runtime,
/// é resolvido 100% em compile-time (`T` é monomorphizado).
///
/// Dá `panic!` em EOF ou se o parse falhar (mesmo efeito prático de uma
/// exceção Python não tratada encerrando o script). Para não dar panic,
/// use [`try_input`].
pub fn input<T>(prompt: &str) -> T
where
    T: FromStr,
    T::Err: Debug,
{
    match try_input::<T>(prompt) {
        Ok(valor) => valor,
        Err(InputError::Eof) => panic!("EOFError: entrada finalizada (Ctrl+D/Ctrl+Z)"),
        Err(InputError::Io(e)) => panic!("erro de I/O ao ler stdin: {e}"),
        Err(InputError::Parse(e)) => panic!("ValueError: entrada inválida para o tipo pedido ({e:?})"),
    }
}

/// Versão de `input()` que não dá panic: devolve `Result` para você
/// tratar EOF e erro de conversão manualmente — equivalente a
/// `try: ... except (EOFError, ValueError): ...` no Python.
pub fn try_input<T>(prompt: &str) -> Result<T, InputError<T::Err>>
where
    T: FromStr,
{
    let texto = read_raw(prompt).map_err(|e| {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            InputError::Eof
        } else {
            InputError::Io(e)
        }
    })?;

    texto.parse::<T>().map_err(InputError::Parse)
}

/// Macro ergonômica equivalente: `input!()` ou `input!("prompt")`.
/// O tipo é inferido do contexto, igual à função.
#[macro_export]
macro_rules! input {
    () => {
        input("")
    };
    ($prompt:expr) => {
        input($prompt)
    };
}

/// Macro ergonômica: `try_input!()` ou `try_input!("prompt")`.
#[macro_export]
macro_rules! try_input {
    () => {
        try_input("")
    };
    ($prompt:expr) => {
        try_input($prompt)
    };
}

#[cfg(test)]
mod tests {
    // Testa só a lógica de parse (independente de stdin real).
    #[test]
    fn string_sempre_converte() {
        let texto = "ola mundo".to_string();
        let r: Result<String, _> = texto.parse();
        assert_eq!(r.unwrap(), "ola mundo");
    }

    #[test]
    fn inteiro_converte() {
        let r: Result<i32, _> = "42".parse();
        assert_eq!(r.unwrap(), 42);
    }

    #[test]
    fn float_converte() {
        let r: Result<f64, _> = "3.14".parse();
        assert_eq!(r.unwrap(), 3.14);
    }

    #[test]
    fn bool_converte() {
        let r: Result<bool, _> = "true".parse();
        assert_eq!(r.unwrap(), true);
    }

    #[test]
    fn inteiro_invalido_falha() {
        let r: Result<i32, _> = "abc".parse();
        assert!(r.is_err());
    }
}

// ============================================================================
// 1) range() — sequências, sem alocar (Iterator de verdade, não Vec)
// ============================================================================
pub mod range_utils {
    //! Equivalente ao `range()` do Python, para inteiros com sinal (`i64`).
    //!
    //! O Rust não tem argumentos opcionais/sobrecarga de função, então em
    //! vez de UMA função "mágica" que aceita 1, 2 ou 3 argumentos (como o
    //! Python faz por baixo dos panos com C variádico), oferecemos três
    //! funções — cada uma cobre uma das formas de chamar `range()`:
    //!
    //! | Python                    | Aqui                              |
    //! |----------------------------|------------------------------------|
    //! | `range(stop)`              | `range(stop)`                       |
    //! | `range(start, stop)`       | `range_from(start, stop)`           |
    //! | `range(start, stop, step)` | `range_step(start, stop, step)`     |
    //!
    //! Por baixo é um `Iterator` de verdade — iterar `range_step(0, 10_000_000, 2)`
    //! não aloca UM byte, diferente de uma lista Python equivalente, que
    //! materializaria a sequência inteira na heap.
    //!
    //! `PyRange` também implementa `ExactSizeIterator`, então
    //! `len(&range(10))` funciona igual a `len(range(10))` do Python — o
    //! tamanho é calculado por aritmética (`O(1)`), sem consumir o
    //! iterador.

    use super::builtins::PyLen;

    /// Iterador equivalente ao `range()` do Python (aceita step negativo).
    pub struct PyRange {
        atual: i64,
        fim: i64,
        passo: i64,
    }

    impl Iterator for PyRange {
        type Item = i64;

        #[inline]
        fn next(&mut self) -> Option<i64> {
            if self.passo > 0 {
                if self.atual >= self.fim {
                    return None;
                }
            } else if self.atual <= self.fim {
                return None;
            }
            let valor = self.atual;
            self.atual += self.passo;
            Some(valor)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            let n = ExactSizeIterator::len(self);
            (n, Some(n))
        }
    }

    /// Tamanho calculado por aritmética, sem iterar — igual a
    /// `len(range(...))` do Python. Usa `i128` internamente só pra não
    /// estourar em faixas beirando os limites de `i64`.
    impl ExactSizeIterator for PyRange {
        fn len(&self) -> usize {
            let (atual, fim, passo) = (self.atual as i128, self.fim as i128, self.passo as i128);
            let n = if passo > 0 {
                if atual >= fim { 0 } else { (fim - atual + passo - 1) / passo }
            } else if atual <= fim {
                0
            } else {
                let passo_abs = -passo;
                (atual - fim + passo_abs - 1) / passo_abs
            };
            n.try_into().unwrap_or(usize::MAX)
        }
    }

    /// Permite `len(&minha_range)` (via `builtins::len`), delegando pro
    /// cálculo O(1) de `ExactSizeIterator::len`.
    impl PyLen for PyRange {
        fn py_len(&self) -> usize {
            ExactSizeIterator::len(self)
        }
    }

    /// `range(stop)` — equivalente a `range(stop)` do Python (start=0, step=1).
    pub fn range(stop: i64) -> PyRange {
        PyRange { atual: 0, fim: stop, passo: 1 }
    }

    /// `range_from(start, stop)` — equivalente a `range(start, stop)` do Python.
    pub fn range_from(start: i64, stop: i64) -> PyRange {
        PyRange { atual: start, fim: stop, passo: 1 }
    }

    /// `range_step(start, stop, step)` — equivalente a `range(start, stop, step)`.
    ///
    /// # Panics
    /// Dá panic se `step == 0` — mesmo comportamento do Python (`ValueError`).
    pub fn range_step(start: i64, stop: i64, step: i64) -> PyRange {
        assert!(step != 0, "ValueError: range() com step igual a zero");
        PyRange { atual: start, fim: stop, passo: step }
    }
}

// ============================================================================
// 2) Listas — métodos "estilo Python" sobre Vec<T>, via extension trait
// ============================================================================
pub mod list_utils {
    //! Métodos estilo Python para `Vec<T>`, como *extension trait* — sem
    //! nenhum custo de runtime: no final das contas isso compila para os
    //! mesmos métodos nativos do `Vec`, só com nomes mais próximos do que
    //! você já conhece do Python.
    //!
    //! ⚠️ **Armadilha evitada de propósito:** `Vec<T>` já tem um método
    //! NATIVO chamado `append(&mut self, other: &mut Vec<T>)` (junta duas
    //! listas inteiras). Métodos inerentes sempre vencem métodos de trait
    //! com o mesmo nome na resolução por `.` — ou seja, se eu chamasse meu
    //! método de `append` também, `minha_lista.append(5)` bateria no
    //! método nativo (esperando `&mut Vec<T>`, não um item solto) e o erro
    //! de compilação seria confuso. Por isso o nome é `append_item`.
    //!
    //! `list.insert(i, valor)` do Python já bate 1:1 com `Vec::insert`
    //! nativo do Rust (mesma ordem de argumentos) — não precisa de wrapper.

    pub trait PyList<T> {
        /// Adiciona um único elemento ao final — equivalente a `lista.append(x)`.
        fn append_item(&mut self, valor: T);

        /// Remove e devolve o elemento no índice dado, ou o ÚLTIMO se
        /// `indice` for `None` — equivalente a `lista.pop()` / `lista.pop(i)`.
        /// `None` se o índice estiver fora da faixa (Rust não faz panic
        /// aqui, ao contrário do `IndexError` do Python).
        fn pop_at(&mut self, indice: Option<usize>) -> Option<T>;

        /// Remove a PRIMEIRA ocorrência igual a `valor` — por VALOR, não
        /// por índice (diferente de `Vec::remove(indice)`, que é por
        /// posição). Equivalente a `lista.remove(valor)` do Python.
        /// Devolve `true` se removeu algo.
        fn remove_value(&mut self, valor: &T) -> bool
        where
            T: PartialEq;
    }

    impl<T> PyList<T> for Vec<T> {
        #[inline]
        fn append_item(&mut self, valor: T) {
            self.push(valor);
        }

        fn pop_at(&mut self, indice: Option<usize>) -> Option<T> {
            match indice {
                None => self.pop(),
                Some(i) if i < self.len() => Some(self.remove(i)),
                Some(_) => None,
            }
        }

        fn remove_value(&mut self, valor: &T) -> bool
        where
            T: PartialEq,
        {
            match self.iter().position(|x| x == valor) {
                Some(pos) => {
                    self.remove(pos);
                    true
                }
                None => false,
            }
        }
    }
}

// ============================================================================
// 3) Matriz — dados contíguos (cache-friendly em x86-64), não Vec<Vec<T>>
// ============================================================================
pub mod matrix_utils {
    //! Matriz densa, com armazenamento CONTÍGUO (`Vec<T>` "achatado"),
    //! em vez de `Vec<Vec<T>>` — que é o que uma lista de listas em Python
    //! vira quando traduzida ingenuamente.
    //!
    //! Isso não é só estética: numa CPU x86-64, `Vec<Vec<T>>` significa
    //! uma alocação separada por linha, espalhadas pela heap — cada
    //! `m[i][j]` pode ser um cache miss (a linha de cache de 64 bytes não
    //! ajuda, porque as linhas nem estão vizinhas na memória). Com um
    //! único `Vec<T>` contíguo, percorrer a matriz sequencialmente
    //! aproveita localidade espacial de verdade (e abre caminho pra
    //! auto-vetorização pelo LLVM no futuro).

    #[derive(Debug, Clone)]
    pub struct Matrix<T> {
        dados: Vec<T>,
        pub linhas: usize,
        pub colunas: usize,
    }

    impl<T: Clone> Matrix<T> {
        /// Equivalente a `[[valor]*colunas for _ in range(linhas)]`.
        pub fn new(linhas: usize, colunas: usize, valor_padrao: T) -> Self {
            Matrix {
                dados: vec![valor_padrao; linhas * colunas],
                linhas,
                colunas,
            }
        }

        /// Acesso seguro (com bound-check explícito, devolvendo `Option`
        /// em vez de panic) — equivalente ao `try/except IndexError`.
        pub fn get(&self, linha: usize, coluna: usize) -> Option<&T> {
            if linha < self.linhas && coluna < self.colunas {
                Some(&self.dados[linha * self.colunas + coluna])
            } else {
                None
            }
        }

        /// Transposta — matriz NOVA, não modifica a original.
        pub fn transpose(&self) -> Matrix<T> {
            if self.dados.is_empty() {
                return Matrix { dados: Vec::new(), linhas: self.colunas, colunas: self.linhas };
            }
            let mut nova = Matrix::new(self.colunas, self.linhas, self.dados[0].clone());
            for i in 0..self.linhas {
                for j in 0..self.colunas {
                    nova[(j, i)] = self[(i, j)].clone();
                }
            }
            nova
        }

        /// Achata a matriz de volta em um `Vec<T>` — consome a matriz
        /// (sem clone: é uma mudança de "forma", não de dados).
        pub fn flatten(self) -> Vec<T> {
            self.dados
        }
    }

    /// Habilita a sintaxe `m[(i, j)]` para LEITURA.
    impl<T> std::ops::Index<(usize, usize)> for Matrix<T> {
        type Output = T;
        #[inline]
        fn index(&self, (linha, coluna): (usize, usize)) -> &T {
            debug_assert!(linha < self.linhas && coluna < self.colunas, "índice fora da matriz");
            &self.dados[linha * self.colunas + coluna]
        }
    }

    /// Habilita a sintaxe `m[(i, j)] = valor` para ESCRITA.
    impl<T> std::ops::IndexMut<(usize, usize)> for Matrix<T> {
        #[inline]
        fn index_mut(&mut self, (linha, coluna): (usize, usize)) -> &mut T {
            debug_assert!(linha < self.linhas && coluna < self.colunas, "índice fora da matriz");
            &mut self.dados[linha * self.colunas + coluna]
        }
    }
}

// ============================================================================
// 4) Built-ins gerais: len(), min(), max(), sorted(), reversed(), type_of()
// ============================================================================
pub mod builtins {
    //! Tudo aqui é genérico resolvido em COMPILE-TIME (monomorphization),
    //! não dynamic dispatch — ou seja, zero overhead sobre chamar os
    //! métodos nativos "na unha".

    /// `len(x)` — funciona pra qualquer coisa com noção de tamanho
    /// (`&str`, `[T]`, `HashMap<K,V>`, `PyRange`, ...), como função livre.
    pub fn len<T: ?Sized + PyLen>(x: &T) -> usize {
        x.py_len()
    }

    pub trait PyLen {
        fn py_len(&self) -> usize;
    }

    impl PyLen for str {
        /// Conta CARACTERES Unicode (`chars().count()`), não bytes —
        /// igual ao `len()` do Python, e diferente do `str::len()` nativo
        /// do Rust (que conta bytes UTF-8 e pode divergir em texto com
        /// acentos/emoji).
        fn py_len(&self) -> usize {
            self.chars().count()
        }
    }

    impl<T> PyLen for [T] {
        fn py_len(&self) -> usize {
            self.len()
        }
    }

    impl<K, V> PyLen for std::collections::HashMap<K, V> {
        fn py_len(&self) -> usize {
            self.len()
        }
    }

    /// `min(iter)` — `None` se vazio. O Python levanta `ValueError` numa
    /// sequência vazia; preferimos `Option`, que é o idiomático em Rust
    /// pra "pode não ter resultado" (evita mais um `try/except`).
    pub fn min<T, I>(iter: I) -> Option<T>
    where
        I: IntoIterator<Item = T>,
        T: PartialOrd,
    {
        iter.into_iter().fold(None, |menor, x| match menor {
            None => Some(x),
            Some(m) => {
                if x < m {
                    Some(x)
                } else {
                    Some(m)
                }
            }
        })
    }

    /// `max(iter)` — mesma lógica de `min`.
    pub fn max<T, I>(iter: I) -> Option<T>
    where
        I: IntoIterator<Item = T>,
        T: PartialOrd,
    {
        iter.into_iter().fold(None, |maior, x| match maior {
            None => Some(x),
            Some(m) => {
                if x > m {
                    Some(x)
                } else {
                    Some(m)
                }
            }
        })
    }

    /// `sorted(iter)` — `Vec<T>` NOVO, já ordenado; não muta nem consome
    /// a coleção original no lugar (diferente de `.sort()`, que é
    /// in-place). Equivalente exato ao `sorted()` do Python.
    ///
    /// ⚠️ Dá panic em valores incomparáveis (ex.: `NaN` em `f64`) — é
    /// deliberado: Python também não tem uma resposta "certa" pra ordenar
    /// `NaN`, e um panic explícito aqui é preferível a uma ordenação
    /// silenciosamente errada.
    pub fn sorted<T, I>(iter: I) -> Vec<T>
    where
        I: IntoIterator<Item = T>,
        T: PartialOrd,
    {
        let mut v: Vec<T> = iter.into_iter().collect();
        v.sort_by(|a, b| a.partial_cmp(b).expect("valores não comparáveis (ex.: NaN)"));
        v
    }

    /// `reversed(iter)` — iterador reverso, LAZY (não aloca).
    pub fn reversed<I>(iter: I) -> std::iter::Rev<I::IntoIter>
    where
        I: IntoIterator,
        I::IntoIter: DoubleEndedIterator,
    {
        iter.into_iter().rev()
    }

    /// `type_of(&x)` — nome do tipo, só pra debug/aprendizado. Não é uma
    /// API estável do Rust (pode mudar de formato entre versões do
    /// compilador) — não use isso pra lógica de programa, só pra imprimir
    /// e entender o que o compilador inferiu.
    pub fn type_of<T>(_: &T) -> &'static str {
        std::any::type_name::<T>()
    }
}

// ============================================================================
// 5) Aritmética — sum(), sub(), mult(), div(), pow(), root()
// ============================================================================
pub mod arithmetic {
    //! Mesma filosofia de erro do Rust: nada aqui dá panic silencioso em
    //! divisão por zero — os casos perigosos devolvem `Result`, diferente
    //! do runtime error "solto" do Python (`ZeroDivisionError`).
    //!
    //! Implementado com UM trait (`NumeroPy`) + macro `impl_numero_py!`
    //! pra cobrir TODAS as larguras inteiras nativas do Rust — `i8/u8`,
    //! `i16/u16`, `i32/u32`, `i64/u64`, `i128/u128` — mais `f32`/`f64`
    //! (as únicas larguras de ponto flutuante estáveis no Rust; `f16`/
    //! `f128` ainda são *nightly-only*, por isso ficam de fora), sem
    //! reescrever a mesma lógica doze vezes. Isso também é uma forma de
    //! matar boilerplate — só que em compile-time, não em runtime como
    //! um `Number` dinâmico do Python.
    //!
    //! Nota de hardware: `i8/u8` e `i16/u16` não têm registrador dedicado
    //! no x86-64 (a ULA opera em 32/64 bits por baixo, com mascaramento);
    //! `i32/f32` cabem num registrador de 32 bits; `i64/f64/u64` usam a
    //! largura nativa de 64 bits; `i128/u128` exigem DOIS registradores
    //! de 64 bits (um par, tipo `rax:rdx`) e são emulados em software —
    //! escolher o tipo certo aqui não é só estilo, afeta throughput real.
    //!
    //! ⚠️ `pow(base, expoente)` aceita só expoente `u32` (não negativo) —
    //! diferente do `**` do Python, que aceita expoente negativo e
    //! devolve `float`. Para expoente negativo, use `root`/divisão manual;
    //! e para inteiros onde overflow importa, prefira o `checked_pow`
    //! nativo do tipo (aqui, em release, overflow em inteiro é wrapping
    //! silencioso, igual a qualquer operação aritmética nativa do Rust).

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub enum ArithError {
        DivisaoPorZero,
        RaizDeNegativoComIndicePar,
    }

    /// Fornece os elementos neutros (0 e 1) pros tipos numéricos
    /// suportados, junto com as operações aritméticas básicas.
    pub trait NumeroPy:
        Copy
        + PartialEq
        + PartialOrd
        + std::ops::Add<Output = Self>
        + std::ops::Sub<Output = Self>
        + std::ops::Mul<Output = Self>
        + std::ops::Div<Output = Self>
    {
        const ZERO: Self;
        const UM: Self;
    }

    macro_rules! impl_numero_py {
        ($($tipo:ty => $zero:expr, $um:expr);+ $(;)?) => {
            $(
                impl NumeroPy for $tipo {
                    const ZERO: Self = $zero;
                    const UM: Self = $um;
                }
            )+
        };
    }

    impl_numero_py!(
        i8 => 0, 1;
        i16 => 0, 1;
        i32 => 0, 1;
        i64 => 0, 1;
        i128 => 0, 1;
        u8 => 0, 1;
        u16 => 0, 1;
        u32 => 0, 1;
        u64 => 0, 1;
        u128 => 0, 1;
        f32 => 0.0, 1.0;
        f64 => 0.0, 1.0;
    );

    /// `sum(iter)` — soma de uma sequência; `0` se vazia (igual ao Python `sum([])`).
    pub fn sum<T: NumeroPy, I: IntoIterator<Item = T>>(iter: I) -> T {
        iter.into_iter().fold(T::ZERO, |acc, x| acc + x)
    }

    /// `sub(iter)` — subtração sequencial (primeiro - segundo - terceiro...).
    /// `None` se a sequência estiver vazia (não existe "neutro" pra subtração
    /// que faça sentido aqui, diferente da soma).
    pub fn sub<T: NumeroPy, I: IntoIterator<Item = T>>(iter: I) -> Option<T> {
        let mut it = iter.into_iter();
        let primeiro = it.next()?;
        Some(it.fold(primeiro, |acc, x| acc - x))
    }

    /// `mult(iter)` — produtório; `1` se vazia (equivalente a `math.prod([])`).
    pub fn mult<T: NumeroPy, I: IntoIterator<Item = T>>(iter: I) -> T {
        iter.into_iter().fold(T::UM, |acc, x| acc * x)
    }

    /// `div(a, b)` — divisão protegida contra zero no denominador.
    pub fn div<T: NumeroPy>(a: T, b: T) -> Result<T, ArithError> {
        if b == T::ZERO {
            Err(ArithError::DivisaoPorZero)
        } else {
            Ok(a / b)
        }
    }

    /// `pow(base, expoente)` — potenciação para expoente inteiro não
    /// negativo (`u32`), sem checagem de overflow (ver nota do módulo).
    pub fn pow<T: NumeroPy>(base: T, expoente: u32) -> T {
        let mut resultado = T::UM;
        for _ in 0..expoente {
            resultado = resultado * base;
        }
        resultado
    }

    /// `root(x, indice)` — raiz n-ésima (`f64`). Erro explícito pra raiz
    /// de índice par de número negativo (resultado não-real), em vez de
    /// devolver `NaN` silenciosamente como o Python/`f64::powf` fariam.
    pub fn root(x: f64, indice: f64) -> Result<f64, ArithError> {
        if x < 0.0 && (indice as i64) % 2 == 0 {
            return Err(ArithError::RaizDeNegativoComIndicePar);
        }
        Ok(x.signum() * x.abs().powf(1.0 / indice))
    }
}

// ============================================================================
// 6) Strings — split(), strip(), join()
// ============================================================================
pub mod string_utils {
    //! A maioria aqui é ZERO-COPY: devolve `&str` (fatias emprestadas da
    //! string original), sem alocar `String` nova à toa — diferente do
    //! Python, onde strings são imutáveis e qualquer operação já implica
    //! cópia.

    /// `split(s, sep)` — como função livre; fatias emprestadas de `s`.
    pub fn split<'a>(s: &'a str, sep: &str) -> Vec<&'a str> {
        s.split(sep).collect()
    }

    /// `strip(s)` — remove espaços do início/fim; fatia emprestada, sem alocar.
    pub fn strip(s: &str) -> &str {
        s.trim()
    }

    /// `join(sep, iter)` — equivalente a `sep.join(iter)` do Python.
    /// Diferente de `split`/`strip`, aqui SEMPRE aloca (concatenar
    /// implica cópia, em qualquer linguagem).
    pub fn join<I, S>(sep: &str, iter: I) -> String
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        iter.into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect::<Vec<_>>()
            .join(sep)
    }
}

// ============================================================================
// 7) OOP estilo C++ — macro `class!` (struct + impl em UM bloco só)
// ============================================================================
//
// Rust não tem (por design) método declarado dentro do struct — dados e
// comportamento são coisas separadas de propósito (permite múltiplos
// `impl` pro mesmo tipo, `impl Trait for Tipo` em qualquer lugar, etc.).
// Esta macro NÃO muda essa regra do compilador — ela só te deixa ESCREVER
// no estilo C++ (campos + métodos juntos) e EXPANDE pra struct + impl
// separados por baixo, que é o que o rustc realmente vê e verifica com o
// borrow checker normalmente.
//
// Gera automaticamente um construtor `new(campo1, campo2, ...)`, no
// estilo "construtor implícito" do C++ / `__init__` do Python.
//
// ⚠️ Limitações assumidas nesta primeira versão (documentar no README):
// - Todos os campos e métodos saem `pub` (sem controle de visibilidade
//   ainda — dá pra evoluir depois).
// - O struct gerado sempre deriva `Debug + Clone`, então todo campo
//   precisa implementar os dois (limitação nova, documentada aqui —
//   antes só estava documentada de forma implícita).
// - Sem suporte a `extends`/herança ainda (fica pro próximo passo: via
//   composição + `Deref`, ou trait com métodos default).
//
// Exemplo de uso prático (Python à esquerda, C++ ao lado, pra comparar):
// ```text
// Python:                          C++:
// class Contador:                  class Contador {
//     def __init__(self, v):       public:
//         self.valor = v               int valor;
//     def incrementar(self):           Contador(int v) : valor(v) {}
//         self.valor += 1               void incrementar() { valor++; }
//                                   };
// ```
// ```
// class! {
//     struct Contador {
//         valor: i32
//     }
//     impl {
//         fn incrementar(&mut self) {
//             self.valor += 1;
//         }
//         fn valor_atual(&self) -> i32 {
//             self.valor
//         }
//     }
// }
// // uso: let mut c = Contador::new(0); c.incrementar();
// ```

#[macro_export]
macro_rules! class {
    (
        struct $nome:ident {
            $( $campo:ident : $tipo:ty ),* $(,)?
        }
        impl {
            $( fn $metodo:ident ( $($params:tt)* ) $(-> $ret:ty)? $corpo:block )*
        }
    ) => {
        #[derive(Debug, Clone)]
        pub struct $nome {
            $( pub $campo: $tipo ),*
        }

        impl $nome {
            /// Construtor gerado automaticamente a partir dos campos.
            pub fn new( $( $campo: $tipo ),* ) -> Self {
                Self { $( $campo ),* }
            }

            $(
                pub fn $metodo( $($params)* ) $(-> $ret)? $corpo
            )*
        }
    };
}

// ============================================================================
// 8) Tempo — time(), sleep(), perf_counter(), timeit(), gmtime()/strftime()
// ============================================================================
pub mod time_utils {
    //! Equivalente (mínimo, só `std`) aos módulos `time` e `datetime` do Python.
    //!
    //! | Python                          | Aqui                                   |
    //! |----------------------------------|-----------------------------------------|
    //! | `time.time()`                    | `time()` → `f64` (epoch, segundos)      |
    //! | `time.sleep(s)`                  | `sleep(s)`                              |
    //! | `time.perf_counter()`            | `perf_counter()` / `perf_counter_ns()`  |
    //! | `timeit.timeit(f, number=n)`     | `timeit(n, \|\| ...)` / `timeit!(n, {...})` |
    //! | `timeit.repeat(f, repeat, n)`    | `timeit_repeat(r, n, \|\| ...)`         |
    //! | `time.gmtime(t)`                 | `gmtime(t)` → `StructTime`              |
    //! | `time.strftime(fmt, t)`          | `strftime(fmt, &t)`                     |
    //! | `calendar.timegm(t)`             | `mktime_utc(&t)`                        |
    //!
    //! ## Uso
    //! ```ignore
    //! use pyrsutils::prelude::*;
    //!
    //! let t0 = perf_counter();
    //! sleep(0.25);
    //! println!("{:.3}s", perf_counter() - t0);
    //!
    //! // Medir um bloco: 1_000 execuções, devolve o TOTAL em segundos.
    //! let total = timeit!(1_000, { (0..100u64).sum::<u64>() });
    //! println!("média: {:.1} ns", total / 1_000.0 * 1e9);
    //!
    //! // Melhor de 5 rodadas (o mínimo é o menos ruidoso — mesma dica do Python):
    //! let melhor = min(timeit_repeat(5, 1_000, || (0..100u64).sum::<u64>())).unwrap();
    //!
    //! // Data/hora UTC:
    //! let agora = gmtime(time() as i64);
    //! println!("{}", strftime("%Y-%m-%d %H:%M:%S %Z", &agora));
    //! ```
    //!
    //! ## Notas de baixo nível (x86-64 / Linux)
    //! - `time()` → `clock_gettime(CLOCK_REALTIME)`; `perf_counter()` →
    //!   `CLOCK_MONOTONIC` (via vDSO: sem syscall real, ~20 ns). É o relógio
    //!   certo para benchmark; o `time()` pode "voltar" com ajuste NTP.
    //! - Nada aqui aloca, exceto `strftime` (1 `String`) e `timeit_repeat`
    //!   (1 `Vec<f64>`).
    //! - `timeit` passa o resultado da closure por `std::hint::black_box`,
    //!   impedindo o LLVM de eliminar o código medido por "dead code".
    //!   Ainda assim, o overhead do laço/relógio entra na medida — para
    //!   ciclos exatos veja `RDTSC` (capítulo 7 do livro de assembly).
    //! - Só UTC: fuso horário local exigiria ler `/etc/localtime` (TZif) —
    //!   fora do escopo sem crate.

    use std::fmt::Write as _;
    use std::hint::black_box;
    use std::sync::OnceLock;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    // ---------------- relógios ----------------

    /// `time.time()` — segundos desde 1970-01-01 UTC, como `f64`.
    /// (Antes de 1970 devolve valor negativo, em vez de dar erro.)
    pub fn time() -> f64 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs_f64(),
            Err(e) => -e.duration().as_secs_f64(),
        }
    }

    /// `time.sleep(secs)` — bloqueia a thread atual.
    ///
    /// # Panics
    /// Se `secs` for negativo, NaN ou infinito (o Python levanta `ValueError`).
    pub fn sleep(secs: f64) {
        let d = Duration::try_from_secs_f64(secs)
            .unwrap_or_else(|_| panic!("ValueError: sleep length inválido ({secs})"));
        std::thread::sleep(d);
    }

    /// Origem (arbitrária, fixa por processo) do `perf_counter`.
    fn origem() -> Instant {
        static ORIGEM: OnceLock<Instant> = OnceLock::new();
        *ORIGEM.get_or_init(Instant::now)
    }

    /// `time.perf_counter()` — relógio MONOTÔNICO em segundos. Só a
    /// diferença entre duas chamadas tem significado.
    pub fn perf_counter() -> f64 {
        origem().elapsed().as_secs_f64()
    }

    /// `time.perf_counter_ns()` — mesmo relógio, em nanossegundos inteiros.
    pub fn perf_counter_ns() -> u64 {
        origem().elapsed().as_nanos() as u64
    }

    // ---------------- timeit ----------------

    /// `timeit.timeit(f, number=n)` — executa `f` `number` vezes e devolve
    /// o tempo TOTAL em segundos. Genérico: `f` pode devolver qualquer `R`
    /// (o valor passa por `black_box`).
    pub fn timeit<R, F: FnMut() -> R>(number: u32, mut f: F) -> f64 {
        let inicio = Instant::now();
        for _ in 0..number {
            black_box(f());
        }
        inicio.elapsed().as_secs_f64()
    }

    /// `timeit.repeat(f, repeat=r, number=n)` — `r` medições de `n` execuções.
    pub fn timeit_repeat<R, F: FnMut() -> R>(repeat: u32, number: u32, mut f: F) -> Vec<f64> {
        (0..repeat).map(|_| timeit(number, &mut f)).collect()
    }

    /// `timeit!(n, { bloco })` → total em segundos (`f64`).
    /// `timeit!({ bloco })` → uma única execução.
    #[macro_export]
    macro_rules! timeit {
        ($body:block) => {
            timeit(1, || $body)
        };
        ($n:expr, $body:block) => {
            timeit($n, || $body)
        };
    }

    // ---------------- calendário UTC ----------------

    /// Equivalente a `time.struct_time` (sempre UTC).
    /// `weekday`: 0 = segunda … 6 = domingo (igual ao Python).
    /// `yearday`: 1..=366.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct StructTime {
        pub year: i64,
        pub month: u32,
        pub day: u32,
        pub hour: u32,
        pub minute: u32,
        pub second: u32,
        pub weekday: u32,
        pub yearday: u32,
    }

    /// Dias desde 1970-01-01 para uma data civil (algoritmo de H. Hinnant;
    /// só aritmética inteira, sem tabelas, sem laços).
    fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
        let y = if m <= 2 { y - 1 } else { y };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let mp = if m > 2 { m as i64 - 3 } else { m as i64 + 9 };
        let doy = (153 * mp + 2) / 5 + d as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// Inverso de `days_from_civil`.
    fn civil_from_days(z: i64) -> (i64, u32, u32) {
        let z = z + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
        (y, m, d)
    }

    /// `time.gmtime(secs)` — epoch (segundos, pode ser negativo) → campos UTC.
    pub fn gmtime(secs: i64) -> StructTime {
        let dias = secs.div_euclid(86_400);
        let resto = secs.rem_euclid(86_400) as u32;
        let (year, month, day) = civil_from_days(dias);
        StructTime {
            year,
            month,
            day,
            hour: resto / 3_600,
            minute: resto % 3_600 / 60,
            second: resto % 60,
            weekday: (dias + 3).rem_euclid(7) as u32, // 1970-01-01 foi quinta (3)
            yearday: (dias - days_from_civil(year, 1, 1) + 1) as u32,
        }
    }

    /// Atalho: `gmtime(time() as i64)`.
    pub fn gmtime_now() -> StructTime {
        gmtime(time().floor() as i64)
    }

    /// `calendar.timegm(t)` — campos UTC → epoch. Inverso de [`gmtime`].
    pub fn mktime_utc(t: &StructTime) -> i64 {
        days_from_civil(t.year, t.month, t.day) * 86_400
            + t.hour as i64 * 3_600
            + t.minute as i64 * 60
            + t.second as i64
    }

    const DIAS: [&str; 7] = [
        "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday",
    ];
    const MESES: [&str; 12] = [
        "January", "February", "March", "April", "May", "June", "July", "August",
        "September", "October", "November", "December",
    ];

    /// `time.strftime(fmt, t)` — diretivas suportadas (locale "C"/inglês):
    /// `%Y %y %m %d %H %M %S %I %p %j %a %A %b %B %Z %F %T %%`.
    /// Diretiva desconhecida é copiada literalmente (como o glibc faz).
    pub fn strftime(fmt: &str, t: &StructTime) -> String {
        let mut out = String::with_capacity(fmt.len() + 16);
        let mut chars = fmt.chars();
        while let Some(c) = chars.next() {
            if c != '%' {
                out.push(c);
                continue;
            }
            // `write!` em String nunca falha → `let _ =` é seguro.
            match chars.next() {
                Some('Y') => { let _ = write!(out, "{}", t.year); }
                Some('y') => { let _ = write!(out, "{:02}", t.year.rem_euclid(100)); }
                Some('m') => { let _ = write!(out, "{:02}", t.month); }
                Some('d') => { let _ = write!(out, "{:02}", t.day); }
                Some('H') => { let _ = write!(out, "{:02}", t.hour); }
                Some('M') => { let _ = write!(out, "{:02}", t.minute); }
                Some('S') => { let _ = write!(out, "{:02}", t.second); }
                Some('I') => {
                    let h = if t.hour % 12 == 0 { 12 } else { t.hour % 12 };
                    let _ = write!(out, "{:02}", h);
                }
                Some('p') => out.push_str(if t.hour < 12 { "AM" } else { "PM" }),
                Some('j') => { let _ = write!(out, "{:03}", t.yearday); }
                Some('a') => out.push_str(&DIAS[t.weekday as usize % 7][..3]),
                Some('A') => out.push_str(DIAS[t.weekday as usize % 7]),
                Some('b') => out.push_str(&MESES[(t.month as usize + 11) % 12][..3]),
                Some('B') => out.push_str(MESES[(t.month as usize + 11) % 12]),
                Some('Z') => out.push_str("UTC"),
                Some('F') => { let _ = write!(out, "{}-{:02}-{:02}", t.year, t.month, t.day); }
                Some('T') => {
                    let _ = write!(out, "{:02}:{:02}:{:02}", t.hour, t.minute, t.second);
                }
                Some('%') => out.push('%'),
                Some(outro) => {
                    out.push('%');
                    out.push(outro);
                }
                None => out.push('%'),
            }
        }
        out
    }
}

// ============================================================================
// 9) Rede — módulo `socket` do Python sobre std::net (TCP, UDP, DNS)
// ============================================================================
pub mod net_utils {
    //! Wrappers finos (custo zero: cada struct só guarda o socket da `std`)
    //! com nomes e semântica próximos do `socket` do Python.
    //!
    //! | Python                         | Aqui                                     |
    //! |---------------------------------|-------------------------------------------|
    //! | `socket.create_connection(a)`   | `TcpClient::connect(a)`                   |
    //! | `s.sendall(b)`                  | `c.send(bytes_ou_str)`                    |
    //! | `s.recv(n)`                     | `c.recv(n)` (vazio = conexão fechada)     |
    //! | `s.settimeout(t)`               | `c.set_timeout(Some(t))`                  |
    //! | `bind()+listen()`               | `TcpServer::listen(a)`                    |
    //! | `srv.accept()`                  | `srv.accept()` → `(TcpClient, SocketAddr)`|
    //! | `SOCK_DGRAM`                    | `UdpSock::bind(a)`; `send_to`/`recv_from` |
    //! | `socket.gethostbyname(h)`       | `gethostbyname(h)` → `Ipv4Addr`           |
    //! | `socket.getaddrinfo(h, p)`      | `getaddrinfo(h, p)` → `Vec<SocketAddr>`   |
    //!
    //! Endereços aceitam qualquer `ToSocketAddrs`: `"127.0.0.1:8080"`,
    //! `("localhost", 8080)`, `"[::1]:80"`. Porta `0` = o SO escolhe uma livre
    //! (use `local_addr()` para descobrir qual).
    //!
    //! ## Uso — servidor de eco + cliente
    //! ```ignore
    //! use pyrsutils::prelude::*;
    //!
    //! // --- servidor (uma thread) ---
    //! let srv = TcpServer::listen("127.0.0.1:0")?;   // porta livre
    //! let porta = srv.local_addr()?.port();
    //! std::thread::spawn(move || {
    //!     let (mut cli, origem) = srv.accept().unwrap();
    //!     println!("conexão de {origem}");
    //!     let dados = cli.recv(1024).unwrap();        // Vec<u8>; vazio = fechou
    //!     cli.send(&dados).unwrap();                  // eco
    //! });
    //!
    //! // --- cliente ---
    //! let mut c = TcpClient::connect(("127.0.0.1", porta))?;
    //! c.set_timeout(Some(2.0))?;                      // 2 s em leitura/escrita
    //! c.send("olá")?;                                 // &str, String, &[u8], Vec<u8>
    //! let resp = c.recv_exact("olá".len())?;
    //! assert_eq!(resp, "olá".as_bytes());
    //!
    //! // --- UDP ---
    //! let a = UdpSock::bind("127.0.0.1:0")?;
    //! let b = UdpSock::bind("127.0.0.1:0")?;
    //! b.send_to("ping", a.local_addr()?)?;
    //! let (msg, de) = a.recv_from(512)?;              // (Vec<u8>, SocketAddr)
    //!
    //! // --- DNS ---
    //! let ip = gethostbyname("localhost")?;           // Ipv4Addr
    //! ```
    //!
    //! ## Notas de baixo nível
    //! - Fechar = `Drop` (RAII, `close(2)` automático). `close()` explícito
    //!   faz `shutdown(2)` antes, enviando FIN mesmo com cópias do fd.
    //! - `recv(n)` aloca um `Vec` de `n` bytes por chamada (heap). Em laços
    //!   quentes use `recv_into(&mut buf)` com buffer reaproveitado (stack ou
    //!   `Vec` único): zero alocação.
    //! - `TcpServer::listen` usa `bind`+`listen` da `std` (backlog 128 e
    //!   `SO_REUSEADDR` ligado no Linux → reiniciar o servidor não dá
    //!   "Address already in use" por `TIME_WAIT`).
    //! - TCP é fluxo de bytes: `recv(n)` pode devolver MENOS que `n`. Para
    //!   tamanho exato, `recv_exact`. UDP preserva fronteiras, mas datagrama
    //!   maior que o buffer é TRUNCADO silenciosamente pelo kernel.
    //! - Tudo é bloqueante (como o `socket` padrão do Python). Para vários
    //!   clientes: uma thread por conexão (`std::thread::spawn`).
    //! - `gethostbyname`/`getaddrinfo` chamam o resolvedor do sistema
    //!   (`getaddrinfo(3)`) e podem bloquear em DNS lento.

    use std::io::{self, Read, Write};
    use std::net::{self, Ipv4Addr, Shutdown, SocketAddr, ToSocketAddrs};
    use std::time::Duration;

    /// `Some(segundos)` → `Some(Duration)`; valida negativo/NaN sem panic.
    fn para_duracao(secs: Option<f64>) -> io::Result<Option<Duration>> {
        match secs {
            None => Ok(None),
            Some(s) => Duration::try_from_secs_f64(s).map(Some).map_err(|_| {
                io::Error::new(io::ErrorKind::InvalidInput, "timeout inválido (negativo/NaN/infinito)")
            }),
        }
    }

    // ---------------- TCP ----------------

    /// Conexão TCP (equivale a um `socket` conectado do Python).
    #[derive(Debug)]
    pub struct TcpClient {
        stream: net::TcpStream,
    }

    impl TcpClient {
        /// Conecta (bloqueante). Tenta todos os IPs que o nome resolver.
        pub fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
            Ok(TcpClient { stream: net::TcpStream::connect(addr)? })
        }

        /// Conecta com limite de tempo por endereço (`socket.create_connection(a, timeout)`).
        pub fn connect_timeout<A: ToSocketAddrs>(addr: A, secs: f64) -> io::Result<Self> {
            let dur = para_duracao(Some(secs))?.unwrap_or_default();
            let mut ultimo = io::Error::new(io::ErrorKind::NotFound, "nenhum endereço resolvido");
            for a in addr.to_socket_addrs()? {
                match net::TcpStream::connect_timeout(&a, dur) {
                    Ok(stream) => return Ok(TcpClient { stream }),
                    Err(e) => ultimo = e,
                }
            }
            Err(ultimo)
        }

        /// `sendall`: envia TUDO (aceita `&str`, `String`, `&[u8]`, `Vec<u8>`...).
        pub fn send<D: AsRef<[u8]>>(&mut self, dados: D) -> io::Result<()> {
            self.stream.write_all(dados.as_ref())
        }

        /// `recv(max)`: até `max` bytes. `Vec` VAZIO = par fechou a conexão.
        pub fn recv(&mut self, max: usize) -> io::Result<Vec<u8>> {
            let mut buf = vec![0u8; max];
            let n = self.stream.read(&mut buf)?;
            buf.truncate(n);
            Ok(buf)
        }

        /// `recv_into`: lê para um buffer SEU (zero alocação). Devolve nº de bytes; 0 = fechou.
        pub fn recv_into(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.stream.read(buf)
        }

        /// Lê EXATAMENTE `n` bytes (erro `UnexpectedEof` se a conexão fechar antes).
        pub fn recv_exact(&mut self, n: usize) -> io::Result<Vec<u8>> {
            let mut buf = vec![0u8; n];
            self.stream.read_exact(&mut buf)?;
            Ok(buf)
        }

        /// Como `recv`, convertendo para `String` (bytes inválidos viram `�`).
        pub fn recv_string(&mut self, max: usize) -> io::Result<String> {
            Ok(String::from_utf8_lossy(&self.recv(max)?).into_owned())
        }

        /// `settimeout`: `None` = bloqueia para sempre; `Some(0.0)` é inválido.
        /// Vale para leitura E escrita.
        pub fn set_timeout(&mut self, secs: Option<f64>) -> io::Result<()> {
            let d = para_duracao(secs)?;
            self.stream.set_read_timeout(d)?;
            self.stream.set_write_timeout(d)
        }

        /// Desliga Nagle (`TCP_NODELAY`): menor latência para mensagens pequenas.
        pub fn set_nodelay(&mut self, ligado: bool) -> io::Result<()> {
            self.stream.set_nodelay(ligado)
        }

        pub fn peer_addr(&self) -> io::Result<SocketAddr> {
            self.stream.peer_addr()
        }

        pub fn local_addr(&self) -> io::Result<SocketAddr> {
            self.stream.local_addr()
        }

        /// Encerra os dois sentidos (`shutdown`) e fecha (consome o cliente).
        pub fn close(self) -> io::Result<()> {
            self.stream.shutdown(Shutdown::Both)
        }
    }

    /// Servidor TCP (junta `bind` + `listen` do Python).
    #[derive(Debug)]
    pub struct TcpServer {
        listener: net::TcpListener,
    }

    impl TcpServer {
        /// `bind` + `listen`. Ex.: `"0.0.0.0:8080"`, `"127.0.0.1:0"` (porta livre).
        pub fn listen<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
            Ok(TcpServer { listener: net::TcpListener::bind(addr)? })
        }

        /// Espera (bloqueia) uma conexão → `(cliente, endereço_do_par)`.
        pub fn accept(&self) -> io::Result<(TcpClient, SocketAddr)> {
            let (stream, addr) = self.listener.accept()?;
            Ok((TcpClient { stream }, addr))
        }

        /// Iterador infinito de conexões: `for cli in srv.incoming() { ... }`.
        pub fn incoming(&self) -> impl Iterator<Item = io::Result<TcpClient>> + '_ {
            self.listener
                .incoming()
                .map(|r| r.map(|stream| TcpClient { stream }))
        }

        pub fn local_addr(&self) -> io::Result<SocketAddr> {
            self.listener.local_addr()
        }
    }

    // ---------------- UDP ----------------

    /// Socket UDP (`SOCK_DGRAM`). Nome `UdpSock` para não colidir com `std::net::UdpSocket`.
    #[derive(Debug)]
    pub struct UdpSock {
        socket: net::UdpSocket,
    }

    impl UdpSock {
        /// `bind`. Cliente que só envia pode usar `"0.0.0.0:0"`.
        pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
            Ok(UdpSock { socket: net::UdpSocket::bind(addr)? })
        }

        /// `sendto`: devolve nº de bytes enviados.
        pub fn send_to<D: AsRef<[u8]>, A: ToSocketAddrs>(&self, dados: D, addr: A) -> io::Result<usize> {
            self.socket.send_to(dados.as_ref(), addr)
        }

        /// `recvfrom(max)`: `(datagrama, origem)`. Excedente de `max` é PERDIDO.
        pub fn recv_from(&self, max: usize) -> io::Result<(Vec<u8>, SocketAddr)> {
            let mut buf = vec![0u8; max];
            let (n, origem) = self.socket.recv_from(&mut buf)?;
            buf.truncate(n);
            Ok((buf, origem))
        }

        /// `recvfrom_into`: buffer SEU, zero alocação → `(nº_bytes, origem)`.
        pub fn recv_from_into(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
            self.socket.recv_from(buf)
        }

        /// `settimeout` (leitura e escrita); `None` = sem limite.
        pub fn set_timeout(&self, secs: Option<f64>) -> io::Result<()> {
            let d = para_duracao(secs)?;
            self.socket.set_read_timeout(d)?;
            self.socket.set_write_timeout(d)
        }

        pub fn local_addr(&self) -> io::Result<SocketAddr> {
            self.socket.local_addr()
        }
    }

    // ---------------- DNS ----------------

    /// `socket.getaddrinfo(host, port)` → todos os endereços (IPv4 e IPv6).
    pub fn getaddrinfo(host: &str, port: u16) -> io::Result<Vec<SocketAddr>> {
        Ok((host, port).to_socket_addrs()?.collect())
    }

    /// `socket.gethostbyname(host)` → primeiro IPv4 (como no Python).
    /// `Err(NotFound)` se o nome só tiver IPv6.
    pub fn gethostbyname(host: &str) -> io::Result<Ipv4Addr> {
        (host, 0)
            .to_socket_addrs()?
            .find_map(|a| match a {
                SocketAddr::V4(v4) => Some(*v4.ip()),
                SocketAddr::V6(_) => None,
            })
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::NotFound, format!("nenhum IPv4 para '{host}'"))
            })
    }
}


// ============================================================================
// Prelude — import único e ergonômico: `use pyrsutils::prelude::*;`
// ============================================================================
pub mod prelude {
    // `super::` (não `crate::`) de propósito: assim o prelude funciona
    // tanto se este arquivo for a raiz do crate (`pyrsutils.rs` como
    // `src/lib.rs`) quanto se for importado como submódulo
    // (`mod pyrsutils;`) dentro de outro projeto.
    pub use super::arithmetic::{div, mult, pow, root, sub, sum, ArithError};
    pub use super::builtins::{len, max, min, reversed, sorted, type_of, PyLen};
    pub use super::list_utils::PyList;
    pub use super::matrix_utils::Matrix;
    pub use super::range_utils::{range, range_from, range_step, PyRange};
    pub use super::string_utils::{join, split, strip};
    pub use super::{input, try_input, InputError};
    // v1.1 — tempo e rede
    pub use super::net_utils::{gethostbyname, getaddrinfo, TcpClient, TcpServer, UdpSock};
    pub use super::time_utils::{
        gmtime, gmtime_now, mktime_utc, perf_counter, perf_counter_ns, sleep, strftime, time,
        timeit, timeit_repeat, StructTime,
    };
    // v1.2 — sistema de arquivos
    pub use super::fs_utils::{
        abspath, basename, chdir, copy, dirname, exists, expanduser, getcwd, glob, isdir, isfile,
        islink, join_path, listdir, makedirs, mkdir, readlink, realpath, remove, rename, rmdir,
        rmtree, splitext, walk, Walk, WalkItem,
    };
    #[cfg(unix)]
    pub use super::fs_utils::{chmod, stat, Stat, symlink};

    // As macros `input!`, `try_input!` e `class!` já são `#[macro_export]`
    // — ficam disponíveis globalmente a partir da raiz do crate
    // (`crate::input!`, ou direto `input!` dentro do próprio crate) sem
    // precisar de `pub use` aqui; macros e itens vivem em "namespaces"
    // diferentes no Rust, então não há conflito com as funções acima de
    // mesmo nome (`input`/`try_input`).
}

// ============================================================================
// Testes das extensões (não mexem nos testes originais de input/parse)
// ============================================================================
#[cfg(test)]
mod tests_extensoes {
    use super::*;

    #[test]
    fn range_basico_funciona() {
        let v: Vec<i64> = range_utils::range(5).collect();
        assert_eq!(v, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn range_step_negativo_funciona() {
        let v: Vec<i64> = range_utils::range_step(10, 0, -2).collect();
        assert_eq!(v, vec![10, 8, 6, 4, 2]);
    }

    /// Teste novo (v1.0): confirma a correção do item 2 do changelog —
    /// `len()` sobre uma `PyRange` funciona sem iterar, para step
    /// positivo, negativo, e faixa vazia.
    #[test]
    fn range_tem_len_sem_iterar() {
        use builtins::len;

        assert_eq!(len(&range_utils::range(10)), 10);
        assert_eq!(len(&range_utils::range_step(10, 0, -2)), 5);
        assert_eq!(len(&range_utils::range_from(5, 5)), 0); // faixa vazia
        assert_eq!(len(&range_utils::range_from(5, 2)), 0); // step positivo, mas fim < início
    }

    #[test]
    fn lista_append_pop_remove() {
        use list_utils::PyList;
        let mut v = vec![1, 2, 3];
        v.append_item(4);
        assert_eq!(v, vec![1, 2, 3, 4]);
        assert_eq!(v.pop_at(None), Some(4));
        assert_eq!(v.pop_at(Some(0)), Some(1));
        assert_eq!(v, vec![2, 3]);
        assert!(v.remove_value(&3));
        assert_eq!(v, vec![2]);
    }

    #[test]
    fn matriz_get_set_transpose() {
        use matrix_utils::Matrix;
        let mut m = Matrix::new(2, 3, 0);
        m[(0, 0)] = 1;
        m[(1, 2)] = 9;
        assert_eq!(m.get(0, 0), Some(&1));
        assert_eq!(m.get(1, 2), Some(&9));
        let t = m.transpose();
        assert_eq!(t[(0, 0)], 1);
        assert_eq!(t[(2, 1)], 9);
    }

    #[test]
    fn builtins_min_max_sorted_reversed() {
        use builtins::*;
        assert_eq!(min(vec![3, 1, 2]), Some(1));
        assert_eq!(max(vec![3, 1, 2]), Some(3));
        assert_eq!(sorted(vec![3, 1, 2]), vec![1, 2, 3]);
        let r: Vec<i32> = reversed(vec![1, 2, 3]).collect();
        assert_eq!(r, vec![3, 2, 1]);
        assert_eq!(len("olá"), 3);
    }

    #[test]
    fn aritmetica_basica() {
        use arithmetic::*;
        assert_eq!(sum(vec![1, 2, 3]), 6);
        assert_eq!(sub(vec![10, 1, 2]), Some(7));
        assert_eq!(mult(vec![2, 3, 4]), 24);
        assert_eq!(div(10, 2), Ok(5));
        assert_eq!(div(10, 0), Err(ArithError::DivisaoPorZero));
        assert_eq!(pow(2, 10), 1024);
        assert!((root(9.0, 2.0).unwrap() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn strings_split_strip_join() {
        use string_utils::*;
        assert_eq!(split("a,b,c", ","), vec!["a", "b", "c"]);
        assert_eq!(strip("  oi  "), "oi");
        assert_eq!(join(", ", vec!["a", "b", "c"]), "a, b, c");
    }

    class! {
        struct Contador {
            valor: i32
        }
        impl {
            fn incrementar(&mut self) {
                self.valor += 1;
            }
            fn valor_atual(&self) -> i32 {
                self.valor
            }
        }
    }

    #[test]
    fn class_macro_gera_struct_e_impl() {
        let mut c = Contador::new(0);
        c.incrementar();
        c.incrementar();
        assert_eq!(c.valor_atual(), 2);
    }
}

// ============================================================================
// Testes v1.1 — Tempo e Rede (loopback local, sem internet)
// ============================================================================
#[cfg(test)]
mod tests_tempo_rede {
    use super::net_utils::*;
    use super::time_utils::*;

    #[test]
    fn time_e_perf_counter_avancam() {
        assert!(time() > 1.7e9); // depois de 2023
        let t0 = perf_counter();
        sleep(0.02);
        assert!(perf_counter() - t0 >= 0.019);
        let n0 = perf_counter_ns();
        assert!(perf_counter_ns() >= n0);
    }

    #[test]
    #[should_panic(expected = "ValueError")]
    fn sleep_negativo_da_panic() {
        sleep(-1.0);
    }

    #[test]
    fn timeit_executa_n_vezes() {
        let mut cont = 0u32;
        let total = timeit(50, || cont += 1);
        assert_eq!(cont, 50);
        assert!(total >= 0.0);
        assert_eq!(timeit_repeat(3, 10, || 1 + 1).len(), 3);
    }

    #[test]
    fn macro_timeit_funciona() {
        let mut c = 0;
        let t = crate::timeit!(7, { c += 1; });
        assert_eq!(c, 7);
        assert!(t >= 0.0);
        let _ = crate::timeit!({ 2 * 2 });
    }

    #[test]
    fn gmtime_epoch_zero() {
        let t = gmtime(0);
        assert_eq!((t.year, t.month, t.day, t.hour, t.minute, t.second), (1970, 1, 1, 0, 0, 0));
        assert_eq!(t.weekday, 3); // quinta
        assert_eq!(t.yearday, 1);
    }

    #[test]
    fn gmtime_valores_conhecidos_e_bissexto() {
        let t = gmtime(1_000_000_000); // 2001-09-09 01:46:40 UTC, domingo
        assert_eq!((t.year, t.month, t.day, t.hour, t.minute, t.second), (2001, 9, 9, 1, 46, 40));
        assert_eq!(t.weekday, 6);
        let b = gmtime(951_782_400); // 2000-02-29, terça, dia 60
        assert_eq!((b.year, b.month, b.day, b.weekday, b.yearday), (2000, 2, 29, 1, 60));
    }

    #[test]
    fn gmtime_negativo_e_ida_e_volta() {
        let t = gmtime(-1); // 1969-12-31 23:59:59
        assert_eq!((t.year, t.month, t.day, t.hour, t.minute, t.second), (1969, 12, 31, 23, 59, 59));
        for s in [-86_400_000_000i64, -1, 0, 59, 951_782_400, 4_102_444_800] {
            assert_eq!(mktime_utc(&gmtime(s)), s);
        }
    }

    #[test]
    fn strftime_diretivas() {
        let t = gmtime(1_000_000_000);
        assert_eq!(strftime("%Y-%m-%d %H:%M:%S", &t), "2001-09-09 01:46:40");
        assert_eq!(strftime("%F %T %Z", &t), "2001-09-09 01:46:40 UTC");
        assert_eq!(strftime("%a %A %b %B %j", &t), "Sun Sunday Sep September 252");
        assert_eq!(strftime("%I:%M %p %y %%", &t), "01:46 AM 01 %");
        assert_eq!(strftime("%Q %", &t), "%Q %"); // desconhecida/pendente: literal
        assert_eq!(strftime("%I %p", &gmtime(0)), "12 AM");
    }

    #[test]
    fn tcp_eco_loopback() {
        let srv = TcpServer::listen("127.0.0.1:0").unwrap();
        let porta = srv.local_addr().unwrap().port();
        let th = std::thread::spawn(move || {
            let (mut cli, _origem) = srv.accept().unwrap();
            let d = cli.recv_exact(5).unwrap();
            cli.send(&d).unwrap();
        });
        let mut c = TcpClient::connect(("127.0.0.1", porta)).unwrap();
        c.set_timeout(Some(2.0)).unwrap();
        c.send("olá!").unwrap(); // 5 bytes em UTF-8
        assert_eq!(c.recv_exact(5).unwrap(), "olá!".as_bytes());
        // Servidor terminou → conexão fecha → recv devolve vazio.
        th.join().unwrap();
        assert!(c.recv(16).unwrap().is_empty());
    }

    #[test]
    fn tcp_connect_timeout_e_erros() {
        let srv = TcpServer::listen("127.0.0.1:0").unwrap();
        let porta = srv.local_addr().unwrap().port();
        assert!(TcpClient::connect_timeout(("127.0.0.1", porta), 1.0).is_ok());
        assert!(TcpClient::connect_timeout(("127.0.0.1", porta), -1.0).is_err());
        let mut c = TcpClient::connect(("127.0.0.1", porta)).unwrap();
        assert!(c.set_timeout(Some(f64::NAN)).is_err());
    }

    #[test]
    fn udp_loopback() {
        let a = UdpSock::bind("127.0.0.1:0").unwrap();
        let b = UdpSock::bind("127.0.0.1:0").unwrap();
        a.set_timeout(Some(2.0)).unwrap();
        let n = b.send_to("ping", a.local_addr().unwrap()).unwrap();
        assert_eq!(n, 4);
        let (msg, de) = a.recv_from(512).unwrap();
        assert_eq!(msg, b"ping");
        assert_eq!(de, b.local_addr().unwrap());
    }

    #[test]
    fn dns_literal_sem_internet() {
        assert_eq!(gethostbyname("127.0.0.1").unwrap(), std::net::Ipv4Addr::LOCALHOST);
        let v = getaddrinfo("127.0.0.1", 80).unwrap();
        assert_eq!(v[0].port(), 80);
    }
}
// ============================================================================
// 10) Sistema de arquivos — os / os.path / pathlib / shutil sobre std::fs
// ============================================================================
pub mod fs_utils {
    //! Equivalente (só `std`, Linux/Unix) a `os`, `os.path`, `pathlib` e
    //! `shutil` do Python.
    //!
    //! | Python                                   | Aqui                          |
    //! |--------------------------------------------|--------------------------------|
    //! | `os.path.exists/isfile/isdir/islink`        | `exists/isfile/isdir/islink`  |
    //! | `os.listdir`                                | `listdir`                     |
    //! | `os.mkdir` / `os.makedirs`                  | `mkdir` / `makedirs`          |
    //! | `os.remove` / `os.rmdir` / `shutil.rmtree`  | `remove` / `rmdir` / `rmtree` |
    //! | `os.rename` / `shutil.copy`                 | `rename` / `copy`             |
    //! | `os.getcwd` / `os.chdir`                    | `getcwd` / `chdir`            |
    //! | `os.path.join/basename/dirname/splitext`    | `join_path/basename/dirname/splitext` |
    //! | `os.path.abspath` / `os.path.realpath`      | `abspath` / `realpath`        |
    //! | `os.path.expanduser`                        | `expanduser`                  |
    //! | `os.walk`                                   | `walk` (iterador preguiçoso)  |
    //! | `glob.glob` (mínimo: `*`/`?`)                | `glob`                        |
    //! | `os.stat` / `os.chmod`                      | `stat` / `chmod`              |
    //! | `os.symlink` / `os.readlink`                | `symlink` / `readlink`        |
    //!
    //! Toda função aceita `AsRef<Path>` — ou seja, `&str`, `String`,
    //! `PathBuf` ou `&Path` funcionam igual, sem conversão manual, do
    //! mesmo jeito que `pathlib`/`os.path` do Python aceitam `str` ou
    //! `Path` intercambiáveis. Nada aqui usa `unwrap()`/panic para erro
    //! de I/O — tudo devolve `io::Result<T>` (equivalente a `OSError`),
    //! seguindo a mesma filosofia do resto da lib.
    //!
    //! `stat`, `chmod` e `symlink` usam `std::os::unix::fs` — por isso só
    //! compilam em Unix (`#[cfg(unix)]`); é a parte da lib pensada
    //! especificamente para Linux x86-64.

    use std::fs;
    use std::io;
    use std::path::{Component, Path, PathBuf};
    #[cfg(unix)]
    use std::os::unix::fs::{symlink as unix_symlink, MetadataExt, PermissionsExt};

    // ---------------- consultas (os.path.exists/isfile/isdir/islink) -------
    pub fn exists<P: AsRef<Path>>(caminho: P) -> bool {
        caminho.as_ref().exists()
    }
    pub fn isfile<P: AsRef<Path>>(caminho: P) -> bool {
        caminho.as_ref().is_file()
    }
    pub fn isdir<P: AsRef<Path>>(caminho: P) -> bool {
        caminho.as_ref().is_dir()
    }
    pub fn islink<P: AsRef<Path>>(caminho: P) -> bool {
        caminho.as_ref().is_symlink()
    }

    // ---------------- listar / criar / apagar (os) -------------------------
    /// `os.listdir` — só os NOMES (não o caminho completo), igual ao Python.
    pub fn listdir<P: AsRef<Path>>(caminho: P) -> io::Result<Vec<String>> {
        fs::read_dir(caminho)?
            .map(|entrada| Ok(entrada?.file_name().to_string_lossy().into_owned()))
            .collect()
    }

    pub fn mkdir<P: AsRef<Path>>(caminho: P) -> io::Result<()> {
        fs::create_dir(caminho)
    }
    /// `os.makedirs` — cria também os diretórios intermediários.
    pub fn makedirs<P: AsRef<Path>>(caminho: P) -> io::Result<()> {
        fs::create_dir_all(caminho)
    }
    pub fn remove<P: AsRef<Path>>(caminho: P) -> io::Result<()> {
        fs::remove_file(caminho)
    }
    pub fn rmdir<P: AsRef<Path>>(caminho: P) -> io::Result<()> {
        fs::remove_dir(caminho)
    }
    /// `shutil.rmtree` — apaga um diretório e TUDO dentro dele.
    pub fn rmtree<P: AsRef<Path>>(caminho: P) -> io::Result<()> {
        fs::remove_dir_all(caminho)
    }
    pub fn rename<P: AsRef<Path>, Q: AsRef<Path>>(de: P, para: Q) -> io::Result<()> {
        fs::rename(de, para)
    }
    /// `shutil.copy` — devolve o nº de bytes copiados (não o caminho, como
    /// o Python; `bytes copiados` é a informação que a `std` já dá de graça).
    pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(de: P, para: Q) -> io::Result<u64> {
        fs::copy(de, para)
    }

    // ---------------- diretório atual (os.getcwd/chdir) --------------------
    pub fn getcwd() -> io::Result<String> {
        Ok(std::env::current_dir()?.to_string_lossy().into_owned())
    }
    pub fn chdir<P: AsRef<Path>>(caminho: P) -> io::Result<()> {
        std::env::set_current_dir(caminho)
    }

    // ---------------- caminho, sem tocar o disco (os.path) -----------------
    /// `os.path.join(*partes)`.
    pub fn join_path(partes: &[&str]) -> String {
        let mut base = PathBuf::new();
        for p in partes {
            base.push(p);
        }
        base.to_string_lossy().into_owned()
    }

    pub fn basename<P: AsRef<Path>>(caminho: P) -> String {
        caminho.as_ref().file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()
    }

    pub fn dirname<P: AsRef<Path>>(caminho: P) -> String {
        caminho.as_ref().parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
    }

    /// `os.path.splitext` — só o ÚLTIMO ponto vira extensão
    /// (`"a.tar.gz"` → `("a.tar", ".gz")`, igual ao Python).
    pub fn splitext<P: AsRef<Path>>(caminho: P) -> (String, String) {
        let p = caminho.as_ref();
        let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let ext = p.extension().map(|s| format!(".{}", s.to_string_lossy())).unwrap_or_default();
        let raiz = match p.parent().filter(|d| !d.as_os_str().is_empty()) {
            Some(dir) => dir.join(&stem).to_string_lossy().into_owned(),
            None => stem,
        };
        (raiz, ext)
    }

    /// `os.path.abspath` — normaliza LEXICAMENTE (resolve `.`/`..` sem
    /// tocar o disco; símlinks NÃO são resolvidos). Para isso, veja
    /// [`realpath`].
    pub fn abspath<P: AsRef<Path>>(caminho: P) -> io::Result<String> {
        let caminho = caminho.as_ref();
        let base = if caminho.is_absolute() {
            caminho.to_path_buf()
        } else {
            std::env::current_dir()?.join(caminho)
        };
        let mut normalizado = PathBuf::new();
        for componente in base.components() {
            match componente {
                Component::ParentDir => {
                    normalizado.pop();
                }
                Component::CurDir => {}
                outro => normalizado.push(outro.as_os_str()),
            }
        }
        Ok(normalizado.to_string_lossy().into_owned())
    }

    /// `os.path.realpath` — absoluto e com símlinks resolvidos (o caminho
    /// PRECISA existir, diferente de `abspath`).
    pub fn realpath<P: AsRef<Path>>(caminho: P) -> io::Result<String> {
        Ok(fs::canonicalize(caminho)?.to_string_lossy().into_owned())
    }

    /// `os.path.expanduser` — só o `~` no INÍCIO do caminho é expandido
    /// (via a variável de ambiente `HOME`), igual ao Python.
    pub fn expanduser(caminho: &str) -> String {
        if let Some(resto) = caminho.strip_prefix('~') {
            if resto.is_empty() || resto.starts_with('/') {
                if let Ok(home) = std::env::var("HOME") {
                    return format!("{home}{resto}");
                }
            }
        }
        caminho.to_string()
    }

    // ---------------- percurso recursivo (os.walk) --------------------------
    /// `(diretório, subdiretórios, arquivos)` — nomes simples, igual ao
    /// `os.walk` do Python.
    pub type WalkItem = (String, Vec<String>, Vec<String>);

    /// Iterador PREGUIÇOSO (top-down, pilha em vez de recursão) equivalente
    /// a `os.walk(dir)`.
    pub struct Walk {
        pilha: Vec<PathBuf>,
    }

    pub fn walk<P: AsRef<Path>>(raiz: P) -> Walk {
        Walk { pilha: vec![raiz.as_ref().to_path_buf()] }
    }

    impl Iterator for Walk {
        type Item = io::Result<WalkItem>;

        fn next(&mut self) -> Option<Self::Item> {
            let dir = self.pilha.pop()?;
            let entradas = match fs::read_dir(&dir) {
                Ok(e) => e,
                Err(erro) => return Some(Err(erro)),
            };

            let mut subdirs = Vec::new();
            let mut arquivos = Vec::new();
            for entrada in entradas {
                let entrada = match entrada {
                    Ok(e) => e,
                    Err(erro) => return Some(Err(erro)),
                };
                let nome = entrada.file_name().to_string_lossy().into_owned();
                if entrada.path().is_dir() {
                    self.pilha.push(entrada.path());
                    subdirs.push(nome);
                } else {
                    arquivos.push(nome);
                }
            }

            Some(Ok((dir.to_string_lossy().into_owned(), subdirs, arquivos)))
        }
    }

    // ---------------- glob mínimo (fnmatch: só * e ?) -----------------------
    fn glob_casa(padrao: &[char], nome: &[char]) -> bool {
        match (padrao.first(), nome.first()) {
            (None, None) => true,
            (Some('*'), _) => {
                glob_casa(&padrao[1..], nome) || (!nome.is_empty() && glob_casa(padrao, &nome[1..]))
            }
            (Some('?'), Some(_)) => glob_casa(&padrao[1..], &nome[1..]),
            (Some(p), Some(n)) if p == n => glob_casa(&padrao[1..], &nome[1..]),
            _ => false,
        }
    }

    /// `glob.glob(padrao)` mínimo: só `*` e `?`, sem `**` recursivo — a
    /// parte antes da última `/` é usada como diretório literal (sem
    /// wildcard), só o último segmento é comparado por padrão.
    pub fn glob(padrao: &str) -> io::Result<Vec<String>> {
        let caminho_padrao = Path::new(padrao);
        let dir = caminho_padrao.parent().filter(|p| !p.as_os_str().is_empty());
        let nome_padrao: Vec<char> = caminho_padrao
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
            .chars()
            .collect();

        let dir_busca = dir.map(|d| d.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
        let mut resultado = Vec::new();
        for entrada in fs::read_dir(&dir_busca)? {
            let entrada = entrada?;
            let nome = entrada.file_name().to_string_lossy().into_owned();
            let nome_chars: Vec<char> = nome.chars().collect();
            if glob_casa(&nome_padrao, &nome_chars) {
                resultado.push(match dir {
                    Some(d) => d.join(&nome).to_string_lossy().into_owned(),
                    None => nome,
                });
            }
        }
        Ok(resultado)
    }

    // ---------------- stat / permissões / links (Linux/Unix) ---------------
    /// Subconjunto de `os.stat_result`: tamanho, mtime (epoch, segundos) e
    /// modo (permissões octais — ex.: `0o644`).
    #[cfg(unix)]
    #[derive(Debug, Clone, Copy)]
    pub struct Stat {
        pub size: u64,
        pub mtime: i64,
        pub mode: u32,
    }

    #[cfg(unix)]
    pub fn stat<P: AsRef<Path>>(caminho: P) -> io::Result<Stat> {
        let meta = fs::metadata(caminho)?;
        Ok(Stat { size: meta.size(), mtime: meta.mtime(), mode: meta.mode() & 0o7777 })
    }

    /// `os.chmod(caminho, modo)` — `modo` em octal, ex.: `0o755`.
    #[cfg(unix)]
    pub fn chmod<P: AsRef<Path>>(caminho: P, modo: u32) -> io::Result<()> {
        let mut perms = fs::metadata(&caminho)?.permissions();
        perms.set_mode(modo);
        fs::set_permissions(caminho, perms)
    }

    /// `os.symlink(alvo, link)`.
    #[cfg(unix)]
    pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(alvo: P, link: Q) -> io::Result<()> {
        unix_symlink(alvo, link)
    }

    /// `os.readlink(link)`.
    pub fn readlink<P: AsRef<Path>>(link: P) -> io::Result<String> {
        Ok(fs::read_link(link)?.to_string_lossy().into_owned())
    }
}

// ============================================================================
// Testes v1.2 — Sistema de arquivos (numa área temporária isolada, sob /tmp)
// ============================================================================
#[cfg(test)]
mod tests_fs {
    use super::fs_utils::*;
    use std::fs;

    fn area_de_teste(sufixo: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("pyrsutils_teste_fs_{}_{}", std::process::id(), sufixo));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn exists_isfile_isdir() {
        let dir = area_de_teste("basico");
        let arq = dir.join("a.txt");
        fs::write(&arq, b"ola").unwrap();
        assert!(exists(&dir));
        assert!(isdir(&dir));
        assert!(isfile(&arq));
        assert!(!isfile(&dir));
        assert!(!exists(dir.join("nao_existe")));
    }

    #[test]
    fn listdir_mkdir_makedirs_remove_rmtree() {
        let dir = area_de_teste("listagem");
        fs::write(dir.join("um.txt"), b"1").unwrap();
        fs::write(dir.join("dois.txt"), b"2").unwrap();
        let mut nomes = listdir(&dir).unwrap();
        nomes.sort();
        assert_eq!(nomes, vec!["dois.txt", "um.txt"]);

        makedirs(dir.join("a/b/c")).unwrap();
        assert!(isdir(dir.join("a/b/c")));

        remove(dir.join("um.txt")).unwrap();
        assert!(!exists(dir.join("um.txt")));

        rmtree(&dir).unwrap();
        assert!(!exists(&dir));
    }

    #[test]
    fn rename_e_copy() {
        let dir = area_de_teste("rename_copy");
        let origem = dir.join("origem.txt");
        fs::write(&origem, b"conteudo").unwrap();
        let destino = dir.join("destino.txt");
        let bytes = copy(&origem, &destino).unwrap();
        assert_eq!(bytes, 8);
        assert_eq!(fs::read_to_string(&destino).unwrap(), "conteudo");

        let renomeado = dir.join("renomeado.txt");
        rename(&destino, &renomeado).unwrap();
        assert!(!exists(&destino));
        assert!(exists(&renomeado));
    }

    #[test]
    fn caminhos_join_basename_dirname_splitext() {
        assert_eq!(join_path(&["/tmp", "a", "b.txt"]), "/tmp/a/b.txt");
        assert_eq!(basename("/tmp/a/b.txt"), "b.txt");
        assert_eq!(dirname("/tmp/a/b.txt"), "/tmp/a");
        let (raiz, ext) = splitext("/tmp/a/arquivo.tar.gz");
        assert_eq!(raiz, "/tmp/a/arquivo.tar");
        assert_eq!(ext, ".gz");
    }

    #[test]
    fn abspath_normaliza_sem_tocar_disco() {
        let cwd = std::env::current_dir().unwrap();
        let resultado = abspath("a/../b/./c").unwrap();
        assert_eq!(resultado, cwd.join("b/c").to_string_lossy());
        assert_eq!(abspath("/x/y/../z").unwrap(), "/x/z");
    }

    #[test]
    fn expanduser_expande_til_inicial() {
        if let Ok(home) = std::env::var("HOME") {
            assert_eq!(expanduser("~/docs"), format!("{home}/docs"));
            assert_eq!(expanduser("~"), home);
        }
        assert_eq!(expanduser("/etc/~raro"), "/etc/~raro");
    }

    #[test]
    fn getcwd_e_chdir_funcionam() {
        let original = getcwd().unwrap();
        let dir = area_de_teste("chdir");
        chdir(&dir).unwrap();
        fs::write("marcador.txt", b"x").unwrap();
        assert!(exists(dir.join("marcador.txt")));
        chdir(&original).unwrap();
    }

    #[test]
    fn walk_percorre_arvore() {
        let dir = area_de_teste("walk");
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("raiz.txt"), b"x").unwrap();
        fs::write(dir.join("sub/folha.txt"), b"y").unwrap();

        let mut total_arquivos = 0;
        for item in walk(&dir) {
            let (_caminho, _subdirs, arquivos) = item.unwrap();
            total_arquivos += arquivos.len();
        }
        assert_eq!(total_arquivos, 2);
    }

    #[test]
    fn glob_minimo_com_asterisco_e_interrogacao() {
        let dir = area_de_teste("glob");
        fs::write(dir.join("a1.txt"), b"").unwrap();
        fs::write(dir.join("a2.txt"), b"").unwrap();
        fs::write(dir.join("b.log"), b"").unwrap();

        let padrao = dir.join("a?.txt");
        let achados = glob(padrao.to_str().unwrap()).unwrap();
        assert_eq!(achados.len(), 2);

        let padrao_estrela = dir.join("*.txt");
        let achados2 = glob(padrao_estrela.to_str().unwrap()).unwrap();
        assert_eq!(achados2.len(), 2);
    }

    #[test]
    fn stat_chmod_symlink_readlink() {
        let dir = area_de_teste("stat_link");
        let arq = dir.join("arquivo.txt");
        fs::write(&arq, b"0123456789").unwrap();

        let info = stat(&arq).unwrap();
        assert_eq!(info.size, 10);

        chmod(&arq, 0o600).unwrap();
        let info2 = stat(&arq).unwrap();
        assert_eq!(info2.mode, 0o600);

        let link = dir.join("link.txt");
        symlink(&arq, &link).unwrap();
        assert!(islink(&link));
        assert_eq!(readlink(&link).unwrap(), arq.to_string_lossy());
    }
}

// ============================================================================
// Teste adicional — cobertura de larguras inteiras (8/16/32/64/128 bits)
// ============================================================================
#[cfg(test)]
mod tests_arithmetic_larguras {
    use super::arithmetic::*;

    #[test]
    fn todas_as_larguras_inteiras_funcionam() {
        assert_eq!(sum(vec![1i8, 2, 3]), 6i8);
        assert_eq!(sum(vec![1u8, 2, 3]), 6u8);
        assert_eq!(sum(vec![1i16, 2, 3]), 6i16);
        assert_eq!(sum(vec![1u16, 2, 3]), 6u16);
        assert_eq!(sum(vec![1i128, 2, 3]), 6i128);
        assert_eq!(sum(vec![1u128, 2, 3]), 6u128);

        assert_eq!(mult(vec![2i8, 3, 4]), 24i8);
        assert_eq!(mult(vec![2u128, 3, 4]), 24u128);

        assert_eq!(div(10i8, 0i8), Err(ArithError::DivisaoPorZero));
        assert_eq!(div(100i128, 5i128), Ok(20i128));

        assert_eq!(pow(2u8, 7), 128u8);
        assert_eq!(pow(2u128, 100), 1267650600228229401496703205376u128);
    }
}