const UNIFORM: [f64; 100] = [0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000, 0.0100000];
const BETA_1: [f64; 100] = [0.028911, 0.048656, 0.061306, 0.068539, 0.071703, 0.071877, 0.069915, 0.066489, 0.062117, 0.057199, 0.052034, 0.046844, 0.041786, 0.036971, 0.032470, 0.028323, 0.024549, 0.021153, 0.018124, 0.015446, 0.013096, 0.011048, 0.009275, 0.007750, 0.006445, 0.005336, 0.004397, 0.003606, 0.002945, 0.002394, 0.001937, 0.001560, 0.001250, 0.000998, 0.000792, 0.000626, 0.000492, 0.000385, 0.000300, 0.000232, 0.000179, 0.000137, 0.000104, 0.000079, 0.000060, 0.000045, 0.000033, 0.000024, 0.000018, 0.000013, 0.000009, 0.000007, 0.000005, 0.000003, 0.000002, 0.000002, 0.000001, 0.000001, 0.000001, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000];
const BETA_2: [f64; 100] = [0.012614, 0.022715, 0.030646, 0.036712, 0.041184, 0.044301, 0.046277, 0.047298, 0.047528, 0.047110, 0.046171, 0.044817, 0.043144, 0.041231, 0.039147, 0.036950, 0.034690, 0.032406, 0.030133, 0.027898, 0.025722, 0.023624, 0.021614, 0.019704, 0.017900, 0.016205, 0.014621, 0.013148, 0.011785, 0.010530, 0.009377, 0.008324, 0.007366, 0.006497, 0.005712, 0.005005, 0.004372, 0.003806, 0.003302, 0.002855, 0.002460, 0.002112, 0.001806, 0.001539, 0.001307, 0.001105, 0.000931, 0.000781, 0.000652, 0.000542, 0.000449, 0.000370, 0.000303, 0.000247, 0.000201, 0.000162, 0.000130, 0.000104, 0.000082, 0.000065, 0.000051, 0.000039, 0.000030, 0.000023, 0.000018, 0.000013, 0.000010, 0.000007, 0.000005, 0.000004, 0.000003, 0.000002, 0.000001, 0.000001, 0.000001, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000];
const BETA_3: [f64; 100] = [0.006715, 0.012509, 0.017463, 0.021655, 0.025155, 0.028031, 0.030344, 0.032151, 0.033506, 0.034458, 0.035052, 0.035331, 0.035332, 0.035092, 0.034643, 0.034015, 0.033234, 0.032327, 0.031314, 0.030217, 0.029054, 0.027841, 0.026592, 0.025322, 0.024042, 0.022761, 0.021489, 0.020234, 0.019002, 0.017799, 0.016630, 0.015499, 0.014409, 0.013362, 0.012361, 0.011407, 0.010500, 0.009641, 0.008830, 0.008067, 0.007351, 0.006681, 0.006056, 0.005475, 0.004936, 0.004437, 0.003978, 0.003555, 0.003168, 0.002814, 0.002492, 0.002199, 0.001934, 0.001695, 0.001481, 0.001288, 0.001116, 0.000963, 0.000828, 0.000708, 0.000603, 0.000511, 0.000431, 0.000361, 0.000301, 0.000250, 0.000206, 0.000168, 0.000137, 0.000110, 0.000088, 0.000070, 0.000055, 0.000043, 0.000033, 0.000025, 0.000019, 0.000014, 0.000010, 0.000007, 0.000005, 0.000004, 0.000002, 0.000002, 0.000001, 0.000001, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000];
const BETA_4: [f64; 100] = [0.003996, 0.007596, 0.010824, 0.013703, 0.016255, 0.018501, 0.020460, 0.022153, 0.023597, 0.024809, 0.025807, 0.026607, 0.027223, 0.027671, 0.027963, 0.028114, 0.028135, 0.028038, 0.027834, 0.027535, 0.027149, 0.026687, 0.026157, 0.025567, 0.024926, 0.024240, 0.023517, 0.022763, 0.021983, 0.021184, 0.020371, 0.019548, 0.018719, 0.017890, 0.017062, 0.016241, 0.015428, 0.014627, 0.013839, 0.013068, 0.012315, 0.011582, 0.010870, 0.010181, 0.009515, 0.008874, 0.008258, 0.007668, 0.007103, 0.006565, 0.006053, 0.005567, 0.005107, 0.004673, 0.004264, 0.003880, 0.003521, 0.003185, 0.002872, 0.002581, 0.002312, 0.002064, 0.001835, 0.001626, 0.001434, 0.001260, 0.001102, 0.000959, 0.000830, 0.000715, 0.000612, 0.000521, 0.000440, 0.000369, 0.000308, 0.000254, 0.000208, 0.000169, 0.000136, 0.000108, 0.000084, 0.000065, 0.000050, 0.000037, 0.000027, 0.000019, 0.000014, 0.000009, 0.000006, 0.000004, 0.000002, 0.000001, 0.000001, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000];
const SINGLE_10: [f64; 100] = [0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 1.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000];
const SINGLE_15: [f64; 100] = [0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 1.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000];
const SINGLE_20: [f64; 100] = [0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 1.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000, 0.00000];

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub enum ThresholdDistribution {
    Uniform,
    Beta1,
    #[default]
    Beta2,
    Beta3,
    Beta4,
    Single10,
    Single15,
    Single20
}

impl ThresholdDistribution {
    pub fn distribution(&self) -> &'static [f64] {
        match self {
            ThresholdDistribution::Uniform => &UNIFORM,
            ThresholdDistribution::Beta1 => &BETA_1,
            ThresholdDistribution::Beta2 => &BETA_2,
            ThresholdDistribution::Beta3 => &BETA_3,
            ThresholdDistribution::Beta4 => &BETA_4,
            ThresholdDistribution::Single10 => &SINGLE_10,
            ThresholdDistribution::Single15 => &SINGLE_15,
            ThresholdDistribution::Single20 => &SINGLE_20
        }
    }
}
