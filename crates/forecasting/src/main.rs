use egui::Ui;
use egui_plot::Legend;

fn main() {
    common::app::run("tool", |cc| {
        let mut parameters = ForecastParameters::default();

        let mut sources = vec![];
        let mut source_id = 0;

        let mut unemployed: Vec<f64> = vec![
            2_967_080.0,
            2_989_220.0,
            2_992_660.0,
            2_806_630.0,
            2_774_030.0,
            2_790_530.0,
            2_806_360.0,
            2_871_910.0,
            2_808_720.0,
            2_726_570.0,
            2_722_550.0,
            2_749_580.0,
            2_769_280.0,
            2_813_810.0,
            2_805_380.0,
            2_636_730.0,
            2_605_730.0,
            2_607_120.0,
            2_627_100.0,
            2_695_830.0,
            2_617_190.0,
            2_554_980.0,
            2_543_740.0,
            2_585_680.0,
            2_593_770.0,
            2_620_170.0,
            2_616_020.0,
            2_453_880.0,
            2_434_020.0,
            2_442_350.0,
            2_485_740.0,
            2_547_340.0,
            2_470_240.0,
            2_362_890.0,
            2_259_650.0,
            2_309_210.0,
            2_362_160.0,
            2_427_960.0,
            2_462_160.0,
            2_329_530.0,
            2_317_070.0,
            2_376_930.0,
            2_464_790.0,
            2_578_470.0,
            2_590_310.0,
            2_613_830.0,
            2_687_190.0,
            2_771_230.0,
            2_827_450.0,
            2_904_410.0,
            2_900_660.0,
            2_707_240.0,
            2_699_130.0,
            2_759_780.0,
            2_847_150.0,
            2_955_490.0,
            2_910_010.0,
            2_853_310.0,
            2_812_990.0,
            2_643_740.0,
            2_335_370.0,
            2_395_600.0,
            2_425_520.0,
            2_227_160.0,
            2_180_000.0,
            2_204_090.0,
            2_234_030.0,
            2_319_410.0,
            2_275_460.0,
            2_216_240.0,
            2_235_970.0,
            2_228_880.0,
            2_301_120.0,
            2_372_700.0,
            2_405_590.0,
            2_209_550.0,
            2_186_110.0,
            2_203_850.0,
            2_256_470.0,
            2_350_880.0,
            2_324_750.0,
            2_275_790.0,
            2_315_490.0,
            2_383_750.0,
            2_458_110.0,
            2_545_940.0,
            2_570_310.0,
            2_384_960.0,
            2_368_410.0,
            2_388_710.0,
            2_448_910.0,
            2_544_850.0,
            2_517_650.0,
            2_472_640.0,
            2_497_720.0,
            2_568_610.0,
            2_662_110.0,
            2_762_100.0,
            2_777_390.0,
            2_568_270.0,
            2_531_980.0,
            2_539_940.0,
            2_607_610.0,
            2_684_290.0,
            2_661_040.0,
            2_614_220.0,
            2_664_010.0,
            2_743_860.0,
            2_844_890.0,
            2_911_170.0,
            2_920_420.0,
            2_681_420.0,
            2_633_160.0,
            2_649_280.0,
            2_708_040.0,
            2_795_600.0,
            2_772_640.0,
            2_711_190.0,
            2_761_700.0,
            2_842_840.0,
            2_931_510.0,
            3_017_000.0,
            3_031_600.0,
            2_763_520.0,
            2_716_850.0,
            2_732_770.0,
            2_807_810.0,
            2_901_820.0,
            2_871_350.0,
            2_832_780.0,
            2_882_030.0,
            2_943_340.0,
            3_054_720.0,
            3_137_870.0,
            3_135_800.0,
            2_873_810.0,
            2_806_150.0,
            2_801_190.0,
            2_848_950.0,
            2_945_710.0,
            2_914_100.0,
            2_864_670.0,
            2_936_920.0,
            3_020_280.0,
            3_097_820.0,
            3_156_250.0,
            3_138_230.0,
            2_839_820.0,
            2_751_490.0,
            2_753_360.0,
            2_788_250.0,
            2_905_110.0,
            2_875_970.0,
            2_809_110.0,
            2_855_270.0,
            2_963_570.0,
            3_028_410.0,
            3_110_440.0,
            3_084_710.0,
            2_780_980.0,
            2_713_810.0,
            2_737_670.0,
            2_796_310.0,
            2_945_550.0,
            2_939_950.0,
            2_893_990.0,
            2_960_620.0,
            3_078_570.0,
            3_211_230.0,
            3_313_210.0,
            3_345_970.0,
            3_011_590.0,
            2_926_970.0,
            2_941_020.0,
            3_026_830.0,
            3_183_220.0,
            3_186_930.0,
            3_148_610.0,
            3_236_360.0,
            3_400_160.0,
            3_560_490.0,
            3_635_350.0,
            3_610_050.0,
            3_268_170.0,
            3_208_040.0,
            3_221_210.0,
            3_338_880.0,
            3_463_440.0,
            3_454_500.0,
            3_402_050.0,
            3_449_220.0,
            3_575_260.0,
            3_576_360.0,
            3_542_600.0,
            3_480_170.0,
            3_094_150.0,
            2_980_970.0,
            2_989_230.0,
            3_073_270.0,
            3_187_760.0,
            3_201_840.0,
            3_151_680.0,
            3_274_030.0,
            3_403_670.0,
            3_496_860.0,
            3_606_080.0,
            3_647_910.0,
            3_395_220.0,
            3_366_880.0,
            3_421_390.0,
            3_530_640.0,
            3_691_890.0,
            3_701_000.0,
            3_672_930.0,
            3_795_940.0,
        ];

        unemployed.reverse();

        sources.push(DemandSource::new("unemployed", unemployed.to_vec()));

        sources.push(DemandSource::new(
            "shampoo",
            vec![
                266.0, 145.9, 183.1, 119.3, 180.3, 168.5, 231.8, 224.5, 192.8, 122.9, 336.5, 185.9,
                194.3, 149.5, 210.1, 273.3, 191.4, 287.0, 226.0, 303.6, 289.9, 421.6, 264.5, 342.3,
                339.7, 440.4, 315.9, 439.3, 401.3, 437.4, 575.5, 407.6, 682.0, 475.3, 581.3, 646.9,
            ],
        ));

        sources.push(DemandSource::new(
            "season test",
            vec![
                14., 10., 6., 2., 18., 8., 4., 1., 16., 9., 5., 3., 18., 11., 4., 2., 17., 9., 5.,
                1.,
            ],
        ));

        let mut forecasts = methods(&sources[0].demand, &parameters);

        let mut show_err = false;

        return Box::new(move |ctx| {
            let ui = ctx.ui;

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    let mut changed = false;
                    changed |= slider_u(ui, &mut parameters.extra_periods, "extra periods");
                    changed |= slider_u(ui, &mut parameters.season_len, "season length");
                    changed |= slider_u(ui, &mut parameters.n, "n");
                    changed |= slider_f(ui, &mut parameters.alpha, "alpha");
                    changed |= slider_f(ui, &mut parameters.beta, "beta");
                    changed |= slider_f(ui, &mut parameters.phi, "phi");
                    changed |= slider_f(ui, &mut parameters.gamma, "gamma");

                    ui.checkbox(&mut show_err, "show err");

                    egui::ComboBox::from_label("demand source")
                        .selected_text(sources[source_id].name.clone())
                        .show_ui(ui, |ui| {
                            for (i, source) in sources.iter().enumerate() {
                                let mut i_temp = source_id;
                                ui.selectable_value(&mut i_temp, i, source.name.clone());
                                if i_temp != source_id {
                                    source_id = i_temp;
                                    changed = true;
                                }
                            }
                        });

                    if changed {
                        for m in forecasts.iter_mut() {
                            m.update(&sources[source_id].demand, &parameters);
                        }
                    }
                });
                ui.vertical(|ui| {
                    for m in forecasts.iter() {
                        ui.label(format!("{}", m.name));
                        ui.label(format!(
                            "Bias {:.0} {:.2} % MAPE {:.2} % MEA {:.0} {:.2} % RSME {:.0} {:.2} %",
                            m.result.bias_abs,
                            m.result.bias_rel * 100.,
                            m.result.mape * 100.,
                            m.result.mae,
                            m.result.mae_rel * 100.,
                            m.result.rmse,
                            m.result.rmse_rel * 100.
                        ));
                    }
                });
            });

            egui_plot::Plot::new("plot")
                .legend(Legend::default().follow_insertion_order(true))
                .show(ui, |plot_ui| {
                    plot_ui.line(egui_plot::Line::new(
                        "demand",
                        egui_plot::PlotPoints::from_ys_f64(&sources[source_id].demand),
                    ));
                    for forcast in forecasts.iter() {
                        if show_err {
                            plot_ui.line(egui_plot::Line::new(
                                format!("{} err", forcast.name),
                                forcast.result.err[1..]
                                    .iter()
                                    .enumerate()
                                    .map(|(i, &y)| [i as f64 + 1., y])
                                    .collect::<egui_plot::PlotPoints>(),
                            ));
                        } else {
                            plot_ui.line(egui_plot::Line::new(
                                forcast.name,
                                forcast.result.forecast[1..]
                                    .iter()
                                    .enumerate()
                                    .map(|(i, &y)| [i as f64 + 1., y])
                                    .collect::<egui_plot::PlotPoints>(),
                            ));
                        }
                    }
                });

        });
    });
}

pub struct DemandSource {
    pub name: String,
    pub demand: Vec<f64>,
}

impl DemandSource {
    pub fn new(name: impl Into<String>, demand: Vec<f64>) -> Self {
        Self {
            name: name.into(),
            demand,
        }
    }
}

pub struct ForecastMethod {
    pub result: ForecastResult,
    pub name: &'static str,
    pub method: &'static dyn Fn(&Vec<f64>, &ForecastParameters) -> Vec<f64>,
}

pub struct ForecastResult {
    pub forecast: Vec<f64>,
    pub err: Vec<f64>,
    pub bias_abs: f64,
    pub bias_rel: f64,
    pub mape: f64,
    pub mae: f64,
    pub mae_rel: f64,
    pub rmse: f64,
    pub rmse_rel: f64,
}

impl ForecastResult {
    pub fn new(
        method: &'static dyn Fn(&Vec<f64>, &ForecastParameters) -> Vec<f64>,
        demand: &Vec<f64>,
        parameters: &ForecastParameters,
    ) -> Self {
        let forecast = (method)(demand, parameters);
        let mut err = vec![];
        for i in 0..forecast.len().min(demand.len()) {
            err.push(forecast[i] - demand[i]);
        }
        let results: Vec<_> = demand
            .clone()
            .into_iter()
            .zip(err.clone())
            .filter(|(x, y)| !x.is_nan() && !y.is_nan())
            .collect();

        let demad: Vec<_> = results.iter().map(|x| x.0).collect();
        let err: Vec<_> = results.iter().map(|x| x.1).collect();

        let mape = results.iter().map(|x| x.1.abs() / x.0).sum::<f64>() / results.len() as f64;

        let mae = results.iter().map(|x| x.1.abs()).sum::<f64>() / results.len() as f64;

        let rmse = (results.iter().map(|x| x.1 * x.1).sum::<f64>() / results.len() as f64).sqrt();

        let dem_ave = mean(&demad);
        let bias_abs = mean(&err);
        let bias_rel = bias_abs / dem_ave;
        let mae_rel = mae / dem_ave;
        let rmse_rel = rmse / dem_ave;
        Self {
            forecast,
            err,
            bias_rel,
            bias_abs,
            mape,
            mae,
            mae_rel,
            rmse,
            rmse_rel,
        }
    }
}

impl ForecastMethod {
    pub fn new(
        name: &'static str,
        method: &'static dyn Fn(&Vec<f64>, &ForecastParameters) -> Vec<f64>,
        demand: &Vec<f64>,
        parameters: &ForecastParameters,
    ) -> Self {
        Self {
            result: ForecastResult::new(method, demand, parameters),
            name,
            method,
        }
    }

    pub fn update(&mut self, demand: &Vec<f64>, parameters: &ForecastParameters) {
        self.result = ForecastResult::new(self.method, demand, parameters);
    }
}

pub fn methods(demand: &Vec<f64>, parameters: &ForecastParameters) -> Vec<ForecastMethod> {
    let mut methods = vec![];

    methods.push(ForecastMethod::new(
        "moving average",
        &|demand, parameters| moving_average(demand, parameters.extra_periods, parameters.n),
        demand,
        parameters,
    ));
    methods.push(ForecastMethod::new(
        "exp smooth",
        &|demand, parameters| exp_smooth(demand, parameters.extra_periods, parameters.alpha),
        demand,
        parameters,
    ));
    methods.push(ForecastMethod::new(
        "double exp smooth",
        &|demand, parameters| {
            double_exp_smooth(
                demand,
                parameters.extra_periods,
                parameters.alpha,
                parameters.beta,
            )
        },
        demand,
        parameters,
    ));
    methods.push(ForecastMethod::new(
        "double exp smooth damped",
        &|demand, parameters| {
            double_exp_smooth_damped(
                demand,
                parameters.extra_periods,
                parameters.alpha,
                parameters.beta,
                parameters.phi,
            )
        },
        demand,
        parameters,
    ));
    methods.push(ForecastMethod::new(
        "triple exp smooth",
        &|demand, parameters| {
            triple_exp_smooth(
                demand,
                parameters.season_len,
                parameters.extra_periods,
                parameters.alpha,
                parameters.beta,
                parameters.phi,
                parameters.gamma,
            )
        },
        demand,
        parameters,
    ));

    return methods;
}

pub struct ForecastParameters {
    pub extra_periods: usize,
    pub season_len: usize,
    pub n: usize,
    pub alpha: f64,
    pub beta: f64,
    pub phi: f64,
    pub gamma: f64,
}

impl Default for ForecastParameters {
    fn default() -> Self {
        Self {
            extra_periods: 10,
            n: 5,
            season_len: 12,
            alpha: 0.4,
            beta: 0.4,
            phi: 0.9,
            gamma: 0.3,
            // season_len: 4,
            // alpha: 0.8,
            // beta: 0.1,
            // phi: 1.0,
            // gamma: 0.4,
        }
    }
}

pub fn slider_u(ui: &mut Ui, value: &mut usize, text: &str) -> bool {
    let mut temp = *value;
    ui.add(egui::Slider::new(&mut temp, 1..=12).text(text));
    if temp != *value {
        *value = temp;
        return true;
    }
    false
}
pub fn slider_f(ui: &mut Ui, value: &mut f64, text: &str) -> bool {
    let mut temp = *value;
    ui.add(egui::Slider::new(&mut temp, 0.0..=1.).text(text));
    if temp != *value {
        *value = temp;
        return true;
    }
    false
}

fn moving_average(demand: &[f64], extra_periods: usize, n: usize) -> Vec<f64> {
    let mut forecast = vec![f64::NAN; demand.len() + extra_periods];

    for i in n..demand.len() {
        forecast[i] = mean(&demand[i - n..i]);
    }

    let avg = mean(&demand[demand.len() - n..]);
    for i in demand.len()..demand.len() + extra_periods {
        forecast[i] = avg;
    }

    forecast
}

fn lin_smooth(alpha: f64, a: f64, b: f64) -> f64 {
    alpha * a + (1. - alpha) * b
}

fn exp_smooth(demand: &[f64], extra_periods: usize, alpha: f64) -> Vec<f64> {
    let mut forecast = vec![f64::NAN; demand.len() + extra_periods];

    forecast[1] = demand[0];

    for i in 2..demand.len() + 1 {
        forecast[i] = lin_smooth(alpha, demand[i - 1], forecast[i - 1]);
    }

    for i in demand.len() + 1..demand.len() + extra_periods {
        forecast[i] = forecast[i - 1];
    }

    forecast
}

fn double_exp_smooth(demand: &[f64], extra_periods: usize, alpha: f64, beta: f64) -> Vec<f64> {
    let mut forecast = vec![f64::NAN; demand.len() + extra_periods];

    let mut a = demand[0];
    let mut b = demand[1] - demand[0];

    for i in 1..demand.len() {
        forecast[i] = a + b;
        let a_new = lin_smooth(alpha, demand[i], a + b);
        b = lin_smooth(beta, a_new - a, b);
        a = a_new;
    }

    for i in demand.len()..demand.len() + extra_periods {
        forecast[i] = a + b;
        a = forecast[i];
        b = b;
    }

    forecast
}

fn double_exp_smooth_damped(
    demand: &[f64],
    extra_periods: usize,
    alpha: f64,
    beta: f64,
    phi: f64,
) -> Vec<f64> {
    let mut forecast = vec![f64::NAN; demand.len() + extra_periods];

    let mut a = demand[0];
    let mut b = demand[1] - demand[0];

    for i in 1..demand.len() {
        forecast[i] = a + phi * b;
        let a_new = lin_smooth(alpha, demand[i], a + phi * b);
        b = lin_smooth(beta, a_new - a, phi * b);
        a = a_new;
    }

    for i in demand.len()..demand.len() + extra_periods {
        forecast[i] = a + phi * b;
        a = forecast[i];
        b = phi * b;
    }

    forecast
}

fn triple_exp_smooth(
    demand: &[f64],
    season_len: usize,
    extra_periods: usize,
    alpha: f64,
    beta: f64,
    phi: f64,
    gamma: f64,
) -> Vec<f64> {
    let mut forecast = vec![f64::NAN; demand.len() + extra_periods];

    let mut seasonal_factors = seasonal_factors(&demand, season_len);

    let mut a = demand[0] / seasonal_factors[0];
    let seasonal_factor1 = if seasonal_factors.len() > 1 {
        seasonal_factors[1]
    } else {
        seasonal_factors[0]
    };
    let mut b = (demand[1] / seasonal_factor1) - (demand[0] / seasonal_factors[0]);

    for i in 1..season_len {
        forecast[i] = (a + phi * b) * seasonal_factors[i];
        let a_new = lin_smooth(alpha, demand[i] / seasonal_factors[i], a + phi * b);
        b = lin_smooth(beta, a_new - a, phi * b);
        a = a_new;
    }

    for i in season_len..demand.len() {
        let last_s = seasonal_factors[i % season_len];

        forecast[i] = (a + phi * b) * last_s;
        let a_new = lin_smooth(alpha, demand[i] / last_s, a + phi * b);
        b = lin_smooth(beta, a_new - a, phi * b);
        seasonal_factors[i % season_len] = lin_smooth(gamma, demand[i] / a_new, last_s);
        a = a_new;
    }

    for i in demand.len()..demand.len() + extra_periods {
        let last_s = seasonal_factors[i % season_len];

        forecast[i] = (a + phi * b) * last_s;
        a = forecast[i] / last_s;
        b = phi * b;
    }

    forecast
}

fn mean(data: &[f64]) -> f64 {
    let sum: f64 = data.iter().sum();
    sum / data.len() as f64
}

fn seasonal_factors(d: &[f64], season_len: usize) -> Vec<f64> {
    let mut seasonal_factors = vec![f64::NAN; season_len];

    for i in 0..season_len {
        let mut sum = 0.0;
        let mut count = 0;

        let mut j = i;
        while j < d.len() {
            sum += d[j];
            count += 1;
            j += season_len;
        }

        seasonal_factors[i] = if count > 0 { sum / count as f64 } else { 0.0 };
    }
    let mean = mean(&seasonal_factors);
    for i in 0..season_len {
        seasonal_factors[i] /= mean;
    }
    return seasonal_factors;
}
