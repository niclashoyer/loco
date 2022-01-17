#[macro_export]
macro_rules! xor {
	( [$x:expr, $( $y:expr ),*] ) => {
		[$x,$( $y ),*,$x $( ^$y )*]
	}
}

#[macro_export]
macro_rules! mov {
	( $b:ident[$p:expr] <- $($x:tt)* ) => {{
		$b[$p].copy_from_slice($($x)*);
		$b[$p].len()
	}};
}
