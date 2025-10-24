use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct Report {
    pub total_return: f64,
    pub max_drawdown: f64,
    pub sharpe: f64,
}

pub fn report(equity_curve: &[f64]) -> Report {
    if equity_curve.is_empty() { return Report::default(); }
    let total_return = *equity_curve.last().unwrap_or(&0.0);
    let max_drawdown = compute_mdd(equity_curve);
    let sharpe = compute_sharpe(equity_curve);
    Report{ total_return, max_drawdown, sharpe }
}

fn compute_mdd(eq: &[f64]) -> f64 {
    let mut peak = f64::MIN; let mut mdd: f64 = 0.0;
    for &v in eq { peak = peak.max(v); mdd = mdd.max(peak - v); }
    mdd
}

fn compute_sharpe(eq: &[f64]) -> f64 {
    if eq.len()<2 { return 0.0; }
    let mut rets = Vec::with_capacity(eq.len()-1);
    for i in 1..eq.len() { rets.push(eq[i]-eq[i-1]); }
    let mean = rets.iter().copied().sum::<f64>() / (rets.len() as f64);
    let var = rets.iter().map(|r| (r-mean)*(r-mean)).sum::<f64>()/(rets.len() as f64);
    if var<=0.0 { 0.0 } else { mean/var.sqrt() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics() {
        let eq = vec![0.0, 1.0, 0.5, 2.0];
        let r = report(&eq);
        assert!(r.total_return > 0.0);
        assert!(r.max_drawdown >= 0.5);
    }
}
