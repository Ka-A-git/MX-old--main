use std::{ops::Add, time::Duration};
pub struct Metrics {
    data: Vec<Duration>,
}

type Result<T> = std::result::Result<T, &'static str>;

impl Metrics {
    pub fn init(mut times: Vec<Duration>) -> Option<Metrics> {
        if times.len() == 0 {
            return None;
        }
        times.sort();

        Some(Metrics { data: times })
    }

    pub fn count(&self) -> usize {
        self.data.len()
    }

    pub fn min(&self) -> Duration {
        *self.data.first().unwrap()
    }

    pub fn max(&self) -> Duration {
        *self.data.last().unwrap()
    }

    pub fn mean(&self) -> Duration {
        self.data
            .iter()
            .sum::<Duration>()
            .div_f64(self.data.len() as f64)
    }

    pub fn median(&self) -> Duration {
        let count = self.data.len();
        match count % 2 {
            // even size
            0 => {
                let first = self.data[count / 2 - 1];
                let second = self.data[count / 2];

                first.add(second).div_f64(2.)
            }
            // odd size
            1 => self.data[count / 2],
            _ => unreachable!(),
        }
    }

    fn percentile25(&self) -> Duration {
        let index = self.data.len() / 4;
        self.data[index]
    }

    fn percentile75(&self) -> Duration {
        let index = 3 * self.data.len() / 4;
        self.data[index]
    }

    fn percentile95(&self) -> Duration {
        let index = 95 * self.data.len() / 100;
        self.data[index]
    }

    fn percentile99(&self) -> Duration {
        let index = 99 * self.data.len() / 100;
        self.data[index]
    }

    pub fn calc(&self) -> String {
        format!("\nCount: {:?}", self.count())
            + &format!("\nMin: {:?}", self.min())
            + &format!("\n25 Percentile: {:?}", self.percentile25())
            + &format!("\nMedian: {:?}", self.median())
            + &format!("\nMean: {:?}", self.mean())
            + &format!("\n75 Percentile: {:?}", self.percentile75())
            + &format!("\n95 Percentile: {:?}", self.percentile95())
            + &format!("\n99 Percentile: {:?}", self.percentile99())
            + &format!("\nMax: {:?}", self.max())
    }
}

fn test_data() -> Vec<Duration> {
    let times = vec![
        Duration::from_micros(1),
        Duration::from_micros(10),
        Duration::from_secs(2),
        Duration::from_secs(20),
        Duration::from_millis(1),
        Duration::from_nanos(50),
        Duration::from_nanos(1),
    ];
    times
}

#[cfg(test)]
mod tests {

    use super::Metrics;
    use crate::platform::metrics::test_data;
    use std::time::Duration;

    fn get_test_metrics() -> Metrics {
        let times = test_data();
        let metrics = Metrics::init(times).unwrap();
        metrics
    }

    #[test]
    fn test_sort() {
        let mut times = test_data();
        times.sort();

        println!("{:?}", times);
    }

    #[test]
    fn init() {
        let times = test_data();
        let _metrics = Metrics::init(times);

        // println!("{:?}", metrics.min());
    }

    #[test]
    fn min() {
        let metrics = get_test_metrics();

        let min = metrics.min();
        assert_eq!(min, Duration::from_nanos(1));
    }

    #[test]
    fn mean() {
        let metrics = get_test_metrics();

        let mean = metrics.mean();
        assert_eq!(mean, Duration::from_nanos(3143001578));
    }

    #[test]
    fn median_even() {
        let times = vec![
            Duration::from_micros(1),
            Duration::from_micros(2),
            Duration::from_micros(3),
            Duration::from_micros(4),
        ];
        let metrics = Metrics::init(times).unwrap();

        let median = metrics.median();

        assert_eq!(median, Duration::from_nanos(2500));
    }

    #[test]
    fn median_odd() {
        let times = vec![
            Duration::from_nanos(1),
            Duration::from_nanos(2),
            Duration::from_nanos(3),
        ];

        let metrics = Metrics::init(times).unwrap();

        let median = metrics.median();
        assert_eq!(median, Duration::from_nanos(2));
        println!("{}", metrics.calc());
    }

    #[test]
    fn percentile25() {
        let metrics = get_test_metrics();

        let percentile25 = metrics.percentile25();
        assert_eq!(percentile25, Duration::from_nanos(50));
    }

    #[test]
    fn percentile75() {
        let metrics = get_test_metrics();

        let percentile75 = metrics.percentile75();
        assert_eq!(percentile75, Duration::from_secs(2));
    }

    #[test]
    fn percentile95() {
        let metrics = get_test_metrics();

        let percentile95 = metrics.percentile95();
        assert_eq!(percentile95, Duration::from_secs(20));
    }

    #[test]
    fn percentile99() {
        let metrics = get_test_metrics();

        let percentile99 = metrics.percentile99();
        assert_eq!(percentile99, Duration::from_secs(20));
    }

    #[test]
    fn max() {
        let metrics = get_test_metrics();

        let max = metrics.max();
        assert_eq!(max, Duration::from_secs(20));
    }
}
