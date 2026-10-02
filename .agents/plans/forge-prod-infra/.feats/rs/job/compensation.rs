struct Salary {
    high: CurrenciedSalary,
    target: CurrenciedSalary,
    low: CurrenciedSalary,
}

type CurrenciedSalary = <&Currency, u32>;
